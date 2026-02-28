use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::db;
use crate::models::{Destination, Job, TransferProgress};
use crate::naming;

const MAX_CONCURRENT: usize = 2;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    jobs: Vec<TransferProgress>,
}

fn update_job_progress(job_id: i64, progress: f64, error: Option<&str>) {
    let status = if error.is_some() {
        "failed"
    } else if progress >= 1.0 {
        "completed"
    } else {
        "transferring"
    };

    let db = db::conn();
    let _ = db.execute(
        "UPDATE jobs SET transfer_progress = ?1, transfer_error = ?2, status = ?3, updated_at = ?4 WHERE id = ?5",
        params![progress, error, status, db::now_iso(), job_id],
    );
}

fn build_relative_path(job: &Job) -> String {
    let group = job
        .group_id
        .and_then(db::get_group_by_id)
        .unwrap_or_else(|| {
            // Fallback for ungrouped jobs
            crate::models::Group {
                id: 0,
                status: "matched".into(),
                media_type: job.media_type.clone(),
                folder_path: String::new(),
                folder_name: String::new(),
                total_file_count: 1,
                total_file_size: job.file_size,
                parsed_title: job.parsed_title.clone(),
                parsed_year: job.parsed_year,
                tmdb_id: job.tmdb_id,
                tmdb_title: job.tmdb_title.clone(),
                tmdb_year: job.tmdb_year,
                tmdb_poster_path: job.tmdb_poster_path.clone(),
                match_confidence: job.match_confidence,
                destination_id: job.destination_id,
                created_at: job.created_at.clone(),
                updated_at: job.updated_at.clone(),
            }
        });

    let settings_map = db::get_all_settings();
    let ns = naming::NamingSettings {
        naming_preset: settings_map
            .get("naming_preset")
            .cloned()
            .unwrap_or_else(|| "jellyfin".into()),
        specials_folder_name: settings_map
            .get("specials_folder_name")
            .cloned()
            .unwrap_or_else(|| "Specials".into()),
        extras_folder_name: settings_map
            .get("extras_folder_name")
            .cloned()
            .unwrap_or_else(|| "Extras".into()),
    };

    naming::format_grouped_path(job, &group, &ns)
}

/// Local file copy with resume support
fn transfer_local(job: &Job, dest: &Destination) -> Result<()> {
    let relative = build_relative_path(job);
    let full_dest = Path::new(&dest.base_path).join(&relative);

    // Create directory structure
    if let Some(parent) = full_dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let total_size = job.file_size as u64;
    let mut transferred: u64 = 0;

    // Check for partial file (resume)
    if full_dest.exists() {
        let existing_size = fs::metadata(&full_dest)?.len();
        if existing_size == total_size {
            update_job_progress(job.id, 1.0, None);
            return Ok(());
        }
        if existing_size < total_size {
            transferred = existing_size;
        }
    }

    let mut reader = fs::File::open(&job.source_path)?;
    if transferred > 0 {
        use std::io::Seek;
        reader.seek(std::io::SeekFrom::Start(transferred))?;
    }

    let mut writer = fs::OpenOptions::new()
        .create(true)
        .append(transferred > 0)
        .write(true)
        .truncate(transferred == 0)
        .open(&full_dest)?;

    let mut buf = vec![0u8; 256 * 1024]; // 256KB buffer
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        transferred += n as u64;
        let progress = (transferred as f64 / total_size as f64).min(1.0);
        update_job_progress(job.id, progress, None);
    }

    update_job_progress(job.id, 1.0, None);

    // Save destination info
    let db = db::conn();
    let _ = db.execute(
        "UPDATE jobs SET destination_id = ?1, destination_path = ?2, updated_at = ?3 WHERE id = ?4",
        params![
            dest.id,
            full_dest.to_string_lossy().to_string(),
            db::now_iso(),
            job.id,
        ],
    );

    Ok(())
}

/// SFTP transfer via ssh2
fn transfer_sftp(job: &Job, dest: &Destination) -> Result<()> {
    let relative = build_relative_path(job);
    let base = dest.base_path.replace('\\', "/");
    let rel = relative.replace('\\', "/");
    let full_dest = format!("{}/{}", base.trim_end_matches('/'), rel);

    let tcp = std::net::TcpStream::connect(format!(
        "{}:{}",
        dest.ssh_host.as_deref().unwrap_or("localhost"),
        dest.ssh_port.unwrap_or(22)
    ))?;
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;

    let mut sess = ssh2::Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    // Authenticate
    if let Some(key_path) = &dest.ssh_key_path {
        let passphrase = dest.ssh_key_passphrase.as_deref();
        sess.userauth_pubkey_file(
            dest.ssh_user.as_deref().unwrap_or("root"),
            None,
            Path::new(key_path),
            passphrase,
        )?;
    } else {
        sess.userauth_agent(dest.ssh_user.as_deref().unwrap_or("root"))?;
    }

    let sftp = sess.sftp()?;

    // Create remote directories
    let dir_path = full_dest.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut current = String::new();
    for part in dir_path.split('/').filter(|p| !p.is_empty()) {
        current.push('/');
        current.push_str(part);
        let _ = sftp.mkdir(Path::new(&current), 0o755);
    }

    // Transfer file
    let total_size = job.file_size as u64;
    let mut transferred: u64 = 0;

    let mut reader = fs::File::open(&job.source_path)?;
    let mut remote_file = sftp.create(Path::new(&full_dest))?;

    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        remote_file.write_all(&buf[..n])?;
        transferred += n as u64;
        let progress = (transferred as f64 / total_size as f64).min(1.0);
        update_job_progress(job.id, progress, None);
    }

    update_job_progress(job.id, 1.0, None);

    // Save destination info
    let db = db::conn();
    let _ = db.execute(
        "UPDATE jobs SET destination_id = ?1, destination_path = ?2, updated_at = ?3 WHERE id = ?4",
        params![dest.id, full_dest, db::now_iso(), job.id],
    );

    Ok(())
}

/// Process a single transfer job
async fn process_transfer(job_id: i64, destination_id: i64) {
    // Mark as transferring
    {
        let db = db::conn();
        let _ = db.execute(
            "UPDATE jobs SET status = 'transferring', transfer_progress = 0, transfer_error = NULL, updated_at = ?1 WHERE id = ?2",
            params![db::now_iso(), job_id],
        );
    }

    let job = {
        let db = db::conn();
        db.query_row(
            "SELECT * FROM jobs WHERE id = ?1",
            params![job_id],
            db::row_to_job,
        )
        .ok()
    };

    let dest = db::get_destination_by_id(destination_id);

    match (job, dest) {
        (Some(job), Some(dest)) => {
            let result = if dest.dest_type == "ssh" {
                // Run blocking SSH in a separate thread
                let j = job.clone();
                let d = dest.clone();
                tokio::task::spawn_blocking(move || transfer_sftp(&j, &d)).await
            } else {
                let j = job.clone();
                let d = dest.clone();
                tokio::task::spawn_blocking(move || transfer_local(&j, &d)).await
            };

            match result {
                Ok(Ok(())) => {
                    info!("Transfer complete for job {}", job_id);
                }
                Ok(Err(e)) => {
                    warn!("Transfer failed for job {}: {}", job_id, e);
                    update_job_progress(job_id, 0.0, Some(&e.to_string()));
                }
                Err(e) => {
                    warn!("Transfer task panic for job {}: {}", job_id, e);
                    update_job_progress(job_id, 0.0, Some(&e.to_string()));
                }
            }
        }
        _ => {
            update_job_progress(job_id, 0.0, Some("Job or destination not found"));
        }
    }
}

/// Queue and run transfers with concurrency limit
pub async fn queue_transfers(
    app: AppHandle,
    job_ids: Vec<i64>,
    destination_id: i64,
) -> Result<usize> {
    let count = job_ids.len();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let app = Arc::new(app);

    // Spawn a background task that emits progress events
    let progress_app = Arc::clone(&app);
    let progress_ids = job_ids.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let progress_jobs: Vec<TransferProgress> = {
                let db = db::conn();
                let placeholders: String = progress_ids
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "SELECT id, status, file_name, file_size, transfer_progress, transfer_error, destination_path FROM jobs WHERE id IN ({})",
                    placeholders
                );
                let mut stmt = match db.prepare(&sql) {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let params: Vec<Box<dyn rusqlite::types::ToSql>> = progress_ids
                    .iter()
                    .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
                    .collect();
                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();
                let result: Vec<TransferProgress> = match stmt.query_map(param_refs.as_slice(), |row| {
                    Ok(TransferProgress {
                        id: row.get("id")?,
                        status: row.get("status")?,
                        file_name: row.get("file_name")?,
                        file_size: row.get("file_size")?,
                        transfer_progress: row.get("transfer_progress")?,
                        transfer_error: row.get("transfer_error")?,
                        destination_path: row.get("destination_path")?,
                    })
                }) {
                    Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                    Err(_) => break,
                };
                result
            };

            let all_done = progress_jobs
                .iter()
                .all(|j| j.status == "completed" || j.status == "failed");

            let _ = progress_app.emit("transfer-progress", &progress_jobs);

            if all_done && !progress_jobs.is_empty() {
                break;
            }
        }
    });

    // Spawn transfer tasks with semaphore
    for job_id in job_ids {
        let sem = Arc::clone(&semaphore);
        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            process_transfer(job_id, destination_id).await;
        });
    }

    Ok(count)
}
