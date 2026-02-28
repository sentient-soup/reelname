use crate::core::naming::{format_grouped_path, NamingPreset};
use crate::db::queries;
use crate::db::schema::*;
use crate::db::DbConn;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, Semaphore};
use tracing::info;

const MAX_CONCURRENT: usize = 2;
const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks for progress reporting

/// Transfer progress update sent to the UI.
#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub job_id: i64,
    pub progress: f64,     // 0.0..1.0
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub status: TransferStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    Transferring,
    Completed,
    Failed,
}

/// Start transferring jobs to a destination.
/// Returns a receiver for progress updates.
pub fn start_transfers(
    conn: DbConn,
    job_ids: Vec<i64>,
    destination_id: i64,
) -> mpsc::UnboundedReceiver<TransferProgress> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let semaphore = std::sync::Arc::new(Semaphore::new(MAX_CONCURRENT));
        let mut handles = Vec::new();

        for job_id in job_ids {
            let conn = conn.clone();
            let tx = tx.clone();
            let sem = semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                transfer_job(conn, job_id, destination_id, tx).await;
            });

            handles.push(handle);
        }

        // Wait for all transfers to complete
        for handle in handles {
            let _ = handle.await;
        }
    });

    rx
}

async fn transfer_job(conn: DbConn, job_id: i64, destination_id: i64, tx: mpsc::UnboundedSender<TransferProgress>) {
    // Fetch job and destination
    let job = match tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || queries::fetch_job(&conn, job_id)
    })
    .await
    {
        Ok(Ok(Some(job))) => job,
        _ => {
            let _ = tx.send(TransferProgress {
                job_id,
                progress: 0.0,
                bytes_transferred: 0,
                total_bytes: 0,
                status: TransferStatus::Failed,
                error: Some("Job not found".to_string()),
            });
            return;
        }
    };

    let dest = match tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || queries::fetch_destination(&conn, destination_id)
    })
    .await
    {
        Ok(Ok(Some(dest))) => dest,
        _ => {
            let _ = tx.send(TransferProgress {
                job_id,
                progress: 0.0,
                bytes_transferred: 0,
                total_bytes: 0,
                status: TransferStatus::Failed,
                error: Some("Destination not found".to_string()),
            });
            return;
        }
    };

    // Update status to transferring
    let _ = tokio::task::spawn_blocking({
        let conn = conn.clone();
        let status = "transferring".to_string();
        let progress: f64 = 0.0;
        let error: Option<String> = None;
        move || {
            queries::update_job(
                &conn,
                job_id,
                &[
                    ("status", &status as &dyn rusqlite::types::ToSql),
                    ("transfer_progress", &progress),
                    ("transfer_error", &error),
                ],
            )
        }
    })
    .await;

    // Build relative path
    let relative_path = build_relative_path(&conn, &job).await;

    // Transfer based on destination type
    let result = match dest.dest_type {
        DestinationType::Local => {
            transfer_local(&job, &dest, &relative_path, &tx).await
        }
        DestinationType::Ssh => {
            transfer_sftp(&job, &dest, &relative_path, &tx).await
        }
    };

    // Update final status
    let (status, progress, error) = match result {
        Ok(_dest_path) => {
            let _ = tx.send(TransferProgress {
                job_id,
                progress: 1.0,
                bytes_transferred: job.file_size as u64,
                total_bytes: job.file_size as u64,
                status: TransferStatus::Completed,
                error: None,
            });
            ("completed".to_string(), 1.0_f64, None::<String>)
        }
        Err(e) => {
            let _ = tx.send(TransferProgress {
                job_id,
                progress: 0.0,
                bytes_transferred: 0,
                total_bytes: job.file_size as u64,
                status: TransferStatus::Failed,
                error: Some(e.clone()),
            });
            ("failed".to_string(), 0.0_f64, Some(e))
        }
    };

    let _ = tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || {
            queries::update_job(
                &conn,
                job_id,
                &[
                    ("status", &status as &dyn rusqlite::types::ToSql),
                    ("transfer_progress", &progress),
                    ("transfer_error", &error),
                ],
            )
        }
    })
    .await;
}

/// Build relative path for a job using naming system.
async fn build_relative_path(conn: &DbConn, job: &Job) -> String {
    let group_id = job.group_id.unwrap_or(0);

    let group = tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || queries::fetch_group(&conn, group_id)
    })
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten();

    // Get naming settings
    let preset_str = tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || queries::get_setting(&conn, "naming_preset")
    })
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten()
    .unwrap_or_else(|| "jellyfin".to_string());

    let specials_folder = tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || queries::get_setting(&conn, "specials_folder_name")
    })
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten()
    .unwrap_or_else(|| "Specials".to_string());

    let extras_folder = tokio::task::spawn_blocking({
        let conn = conn.clone();
        move || queries::get_setting(&conn, "extras_folder_name")
    })
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten()
    .unwrap_or_else(|| "Extras".to_string());

    let preset = NamingPreset::from_str(&preset_str);

    if let Some(group) = &group {
        format_grouped_path(group, job, preset, &specials_folder, &extras_folder)
    } else {
        // Fallback: create synthetic group from job fields
        let synthetic_group = Group {
            id: 0,
            status: GroupStatus::Confirmed,
            media_type: job.media_type,
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
            destination_id: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        format_grouped_path(&synthetic_group, job, preset, &specials_folder, &extras_folder)
    }
}

/// Transfer a file locally with resume support.
async fn transfer_local(
    job: &Job,
    dest: &Destination,
    relative_path: &str,
    tx: &mpsc::UnboundedSender<TransferProgress>,
) -> Result<String, String> {
    let dest_path = PathBuf::from(&dest.base_path).join(relative_path);
    let dest_str = dest_path.to_string_lossy().to_string();

    // Create parent directories
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directories: {e}"))?;
    }

    let source_path = Path::new(&job.source_path);
    let total_size = job.file_size as u64;

    // Check for resume
    let existing_size = if dest_path.exists() {
        tokio::fs::metadata(&dest_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    if existing_size >= total_size && total_size > 0 {
        // Already complete
        info!("Job {} already transferred", job.id);
        return Ok(dest_str);
    }

    // Copy with progress
    use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

    let mut reader = tokio::fs::File::open(source_path)
        .await
        .map_err(|e| format!("Failed to open source: {e}"))?;

    let mut writer = if existing_size > 0 {
        // Resume
        reader
            .seek(std::io::SeekFrom::Start(existing_size))
            .await
            .map_err(|e| format!("Failed to seek: {e}"))?;
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&dest_path)
            .await
            .map_err(|e| format!("Failed to open dest for resume: {e}"))?
    } else {
        tokio::fs::File::create(&dest_path)
            .await
            .map_err(|e| format!("Failed to create dest: {e}"))?
    };

    let mut transferred = existing_size;
    let mut buf = vec![0u8; CHUNK_SIZE];

    loop {
        let n = reader
            .read(&mut buf)
            .await
            .map_err(|e| format!("Read error: {e}"))?;
        if n == 0 {
            break;
        }

        writer
            .write_all(&buf[..n])
            .await
            .map_err(|e| format!("Write error: {e}"))?;

        transferred += n as u64;
        let progress = if total_size > 0 {
            transferred as f64 / total_size as f64
        } else {
            1.0
        };

        let _ = tx.send(TransferProgress {
            job_id: job.id,
            progress,
            bytes_transferred: transferred,
            total_bytes: total_size,
            status: TransferStatus::Transferring,
            error: None,
        });
    }

    writer
        .flush()
        .await
        .map_err(|e| format!("Flush error: {e}"))?;

    info!("Job {} transferred to {}", job.id, dest_str);
    Ok(dest_str)
}

/// Transfer a file via SFTP.
async fn transfer_sftp(
    job: &Job,
    dest: &Destination,
    relative_path: &str,
    tx: &mpsc::UnboundedSender<TransferProgress>,
) -> Result<String, String> {
    // Normalize to forward slashes for remote path
    let base = dest.base_path.replace('\\', "/");
    let rel = relative_path.replace('\\', "/");
    let remote_path = format!("{}/{}", base.trim_end_matches('/'), rel);

    let host = dest.ssh_host.as_deref().ok_or("No SSH host configured")?;
    let port = dest.ssh_port.unwrap_or(22) as u16;
    let user = dest.ssh_user.as_deref().ok_or("No SSH user configured")?;

    // Build SSH config
    let config = russh::client::Config::default();
    let config = std::sync::Arc::new(config);

    // Connect
    let mut session = russh::client::connect(config, (host, port), SshHandler)
        .await
        .map_err(|e| format!("SSH connect failed: {e}"))?;

    // Authenticate
    if let Some(key_path) = &dest.ssh_key_path {
        let key_data = tokio::fs::read_to_string(key_path)
            .await
            .map_err(|e| format!("Failed to read SSH key: {e}"))?;
        let passphrase = dest.ssh_key_passphrase.as_deref();
        let key_pair = russh_keys::decode_secret_key(&key_data, passphrase)
            .map_err(|e| format!("Failed to decode SSH key: {e}"))?;
        let auth = session
            .authenticate_publickey(user, std::sync::Arc::new(key_pair))
            .await
            .map_err(|e| format!("SSH auth failed: {e}"))?;
        if !auth {
            return Err("SSH public key authentication failed".to_string());
        }
    } else {
        return Err("No SSH key path configured".to_string());
    }

    // Open SFTP channel
    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| format!("SSH channel open failed: {e}"))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("SFTP subsystem request failed: {e}"))?;

    let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("SFTP session failed: {e}"))?;

    // Create remote directories
    let remote_dir = if let Some(pos) = remote_path.rfind('/') {
        &remote_path[..pos]
    } else {
        "."
    };
    mkdir_recursive(&sftp, remote_dir).await?;

    // Read source file
    let source_data = tokio::fs::read(&job.source_path)
        .await
        .map_err(|e| format!("Failed to read source: {e}"))?;

    let total_size = source_data.len() as u64;

    // Write to remote
    let mut remote_file = sftp
        .create(&remote_path)
        .await
        .map_err(|e| format!("SFTP create failed: {e}"))?;

    use tokio::io::AsyncWriteExt;
    let mut transferred: u64 = 0;
    for chunk in source_data.chunks(CHUNK_SIZE) {
        remote_file
            .write_all(chunk)
            .await
            .map_err(|e| format!("SFTP write failed: {e}"))?;

        transferred += chunk.len() as u64;
        let progress = if total_size > 0 {
            transferred as f64 / total_size as f64
        } else {
            1.0
        };

        let _ = tx.send(TransferProgress {
            job_id: job.id,
            progress,
            bytes_transferred: transferred,
            total_bytes: total_size,
            status: TransferStatus::Transferring,
            error: None,
        });
    }

    remote_file
        .flush()
        .await
        .map_err(|e| format!("SFTP flush failed: {e}"))?;
    remote_file
        .shutdown()
        .await
        .map_err(|e| format!("SFTP shutdown failed: {e}"))?;

    info!("Job {} SFTP transferred to {}", job.id, remote_path);
    Ok(remote_path)
}

/// Recursively create remote directories via SFTP.
async fn mkdir_recursive(sftp: &russh_sftp::client::SftpSession, path: &str) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut current = String::new();
    for part in parts {
        if current.is_empty() && path.starts_with('/') {
            current = format!("/{part}");
        } else if current.is_empty() {
            current = part.to_string();
        } else {
            current = format!("{current}/{part}");
        }
        // Try to create, ignore errors (dir may already exist)
        let _ = sftp.create_dir(&current).await;
    }
    Ok(())
}

/// Test an SSH connection with the given credentials.
/// Returns Ok with a success message or Err with a descriptive error.
pub async fn test_ssh_connection(
    host: &str,
    port: u16,
    user: &str,
    key_path: &str,
    passphrase: Option<&str>,
) -> Result<String, String> {
    if host.is_empty() {
        return Err("SSH host is required".to_string());
    }
    if user.is_empty() {
        return Err("SSH username is required".to_string());
    }
    if key_path.is_empty() {
        return Err("SSH key path is required".to_string());
    }

    let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let config = std::sync::Arc::new(russh::client::Config::default());

        let mut session = russh::client::connect(config, (host, port), SshHandler)
            .await
            .map_err(|e| format!("Connection failed: {e}"))?;

        let key_data = tokio::fs::read_to_string(key_path)
            .await
            .map_err(|e| format!("Failed to read SSH key: {e}"))?;

        let key_pair = russh_keys::decode_secret_key(&key_data, passphrase)
            .map_err(|e| format!("Failed to decode SSH key: {e}"))?;

        let auth = session
            .authenticate_publickey(user, std::sync::Arc::new(key_pair))
            .await
            .map_err(|e| format!("Authentication failed: {e}"))?;

        if !auth {
            return Err("Public key authentication rejected".to_string());
        }

        Ok("Success: Connected and authenticated".to_string())
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err("Connection timed out after 10 seconds".to_string()),
    }
}

/// Minimal SSH handler for russh.
struct SshHandler;

#[async_trait::async_trait]
impl russh::client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys (like SSH StrictHostKeyChecking=no)
        // In production, this should verify against known_hosts
        Ok(true)
    }
}
