use rusqlite::params;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tauri::AppHandle;

use crate::db;
use crate::matcher;
use crate::models::*;
use crate::naming;
use crate::parser;
use crate::scanner;
use crate::tmdb;
use crate::transfer;

// ── Scan ────────────────────────────────────────────────

#[tauri::command]
pub async fn scan_directory(path: Option<String>) -> Result<ScanResult, String> {
    let scan_path = path
        .or_else(|| db::get_setting("scan_path").filter(|s| !s.is_empty()))
        .ok_or("No scan path configured. Set it in settings.")?;

    // Clean up orphaned jobs
    {
        let db = db::conn();
        let _ = db.execute("DELETE FROM jobs WHERE group_id IS NULL", []);
    }

    let scanned_groups = scanner::scan_directory_grouped(&scan_path);

    // Get existing group folder paths to avoid duplicates
    let existing_paths: HashSet<String> = {
        let db = db::conn();
        let mut stmt = db
            .prepare("SELECT folder_path FROM groups")
            .map_err(|e| e.to_string())?;
        let result: HashSet<String> = stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    let mut added_groups = 0usize;
    let mut added_files = 0usize;
    let mut skipped_groups = 0usize;
    let now = db::now_iso();

    for scanned_group in &scanned_groups {
        if existing_paths.contains(&scanned_group.folder_path) {
            skipped_groups += 1;
            continue;
        }

        let parsed_folder = parser::parse_folder_name(&scanned_group.folder_name);

        // Determine media type
        let has_episodes = scanned_group
            .files
            .iter()
            .any(|f| f.file_category == "episode" || f.file_category == "special");
        let all_movies = scanned_group
            .files
            .iter()
            .all(|f| f.file_category == "movie");
        let media_type = if all_movies {
            "movie"
        } else if has_episodes {
            "tv"
        } else {
            "unknown"
        };

        let total_size: i64 = scanned_group.files.iter().map(|f| f.file_size).sum();

        let group_id: i64 = {
            let db = db::conn();
            db.execute(
                "INSERT INTO groups (status, media_type, folder_path, folder_name, total_file_count, total_file_size, parsed_title, parsed_year, created_at, updated_at)
                 VALUES ('scanned', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    media_type,
                    scanned_group.folder_path,
                    scanned_group.folder_name,
                    scanned_group.files.len() as i64,
                    total_size,
                    parsed_folder.title,
                    parsed_folder.year,
                    now,
                    now,
                ],
            )
            .map_err(|e| e.to_string())?;
            db.last_insert_rowid()
        };

        added_groups += 1;

        // Insert child jobs
        for file in &scanned_group.files {
            let parsed = parser::parse_file_name(&file.file_name);
            let season = file.detected_season.or(parsed.season);
            let episode = parsed.episode;

            // Check if job already exists for this source path
            let existing_job_id: Option<i64> = {
                let db = db::conn();
                db.query_row(
                    "SELECT id FROM jobs WHERE source_path = ?1",
                    params![file.source_path],
                    |row| row.get(0),
                )
                .ok()
            };

            let db = db::conn();
            if let Some(existing_id) = existing_job_id {
                let _ = db.execute(
                    "UPDATE jobs SET group_id = ?1, status = 'scanned', media_type = ?2, file_category = ?3, extra_type = ?4,
                     parsed_title = ?5, parsed_year = ?6, parsed_season = ?7, parsed_episode = ?8, parsed_quality = ?9, parsed_codec = ?10,
                     tmdb_id = NULL, tmdb_title = NULL, tmdb_year = NULL, tmdb_poster_path = NULL, tmdb_episode_title = NULL,
                     match_confidence = NULL, updated_at = ?11 WHERE id = ?12",
                    params![
                        group_id,
                        media_type,
                        file.file_category,
                        file.extra_type,
                        parsed.title,
                        parsed.year,
                        season,
                        episode,
                        parsed.quality,
                        parsed.codec,
                        now,
                        existing_id,
                    ],
                );
            } else {
                let _ = db.execute(
                    "INSERT INTO jobs (group_id, status, media_type, file_category, extra_type, source_path, file_name, file_size, file_extension,
                     parsed_title, parsed_year, parsed_season, parsed_episode, parsed_quality, parsed_codec, created_at, updated_at)
                     VALUES (?1, 'scanned', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        group_id,
                        media_type,
                        file.file_category,
                        file.extra_type,
                        file.source_path,
                        file.file_name,
                        file.file_size,
                        file.file_extension,
                        parsed.title,
                        parsed.year,
                        season,
                        episode,
                        parsed.quality,
                        parsed.codec,
                        now,
                        now,
                    ],
                );
            }

            added_files += 1;
        }
    }

    // Auto-match if TMDB key is configured
    let mut match_result = MatchResult {
        matched: 0,
        ambiguous: 0,
    };
    let mut match_error: Option<String> = None;

    let tmdb_key = db::get_setting("tmdb_api_key");
    if let Some(_key) = tmdb_key.filter(|k| !k.trim().is_empty()) {
        match matcher::match_all_groups().await {
            Ok(result) => match_result = result,
            Err(e) => match_error = Some(e.to_string()),
        }
    } else {
        match_error =
            Some("No TMDB API key configured. Set it in Settings to enable auto-matching.".into());
    }

    Ok(ScanResult {
        scanned_groups: scanned_groups.len(),
        added_groups,
        added_files,
        skipped_groups,
        matched: match_result.matched,
        ambiguous: match_result.ambiguous,
        match_error,
    })
}

// ── Match ───────────────────────────────────────────────

#[tauri::command]
pub async fn match_groups() -> Result<MatchResult, String> {
    let tmdb_key = db::get_setting("tmdb_api_key");
    if tmdb_key.filter(|k| !k.trim().is_empty()).is_none() {
        return Err("No TMDB API key configured. Set it in Settings.".into());
    }

    matcher::match_all_groups().await.map_err(|e| e.to_string())
}

// ── Groups ──────────────────────────────────────────────

#[tauri::command]
pub async fn get_groups(
    status: Option<String>,
    media_type: Option<String>,
    search: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<GroupsResponse, String> {
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(50);
    let sort_by = sort_by.unwrap_or_else(|| "created_at".into());
    let sort_dir = sort_dir.unwrap_or_else(|| "desc".into());

    let sort_column = match sort_by.as_str() {
        "folderName" => "folder_name",
        "totalFileSize" => "total_file_size",
        "totalFileCount" => "total_file_count",
        "status" => "status",
        "mediaType" => "media_type",
        "matchConfidence" => "match_confidence",
        _ => "created_at",
    };
    let order = if sort_dir == "asc" { "ASC" } else { "DESC" };

    // Build WHERE clause
    let mut conditions = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref s) = status {
        param_values.push(Box::new(s.clone()));
        conditions.push(format!("status = ?{}", param_values.len()));
    }
    if let Some(ref mt) = media_type {
        param_values.push(Box::new(mt.clone()));
        conditions.push(format!("media_type = ?{}", param_values.len()));
    }
    if let Some(ref s) = search {
        param_values.push(Box::new(format!("%{}%", s)));
        conditions.push(format!("folder_name LIKE ?{}", param_values.len()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let (total, groups) = {
        let db = db::conn();

        // Count total
        let count_sql = format!("SELECT count(*) FROM groups {}", where_clause);
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let total: i64 = db
            .query_row(&count_sql, param_refs.as_slice(), |row| row.get(0))
            .unwrap_or(0);

        // Fetch groups
        let offset = (page - 1) * limit;
        param_values.push(Box::new(limit));
        param_values.push(Box::new(offset));
        let select_sql = format!(
            "SELECT * FROM groups {} ORDER BY {} {} LIMIT ?{} OFFSET ?{}",
            where_clause,
            sort_column,
            order,
            param_values.len() - 1,
            param_values.len(),
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = db.prepare(&select_sql).map_err(|e| e.to_string())?;
        let groups: Vec<Group> = stmt
            .query_map(param_refs.as_slice(), db::row_to_group)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        (total, groups)
    };

    // Load naming settings for preview paths
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

    let groups_with_jobs: Vec<GroupWithJobs> = groups
        .into_iter()
        .map(|group| {
            let jobs = db::get_jobs_for_group(group.id);
            let jobs_with_preview: Vec<JobWithPreview> = jobs
                .into_iter()
                .map(|job| {
                    let preview = if group.tmdb_id.is_some() {
                        Some(naming::format_grouped_path(&job, &group, &ns))
                    } else {
                        None
                    };
                    JobWithPreview {
                        job,
                        preview_name: preview,
                    }
                })
                .collect();

            GroupWithJobs {
                group,
                jobs: jobs_with_preview,
                candidates: None,
            }
        })
        .collect();

    Ok(GroupsResponse {
        groups: groups_with_jobs,
        total,
        page,
        limit,
    })
}

#[tauri::command]
pub async fn get_group(id: i64) -> Result<GroupWithJobs, String> {
    let group = db::get_group_by_id(id).ok_or("Group not found")?;
    let jobs = db::get_jobs_for_group(id);
    let candidates = db::get_candidates_for_group(id);

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

    let jobs_with_preview: Vec<JobWithPreview> = jobs
        .into_iter()
        .map(|job| {
            let preview = if group.tmdb_id.is_some() {
                Some(naming::format_grouped_path(&job, &group, &ns))
            } else {
                None
            };
            JobWithPreview {
                job,
                preview_name: preview,
            }
        })
        .collect();

    Ok(GroupWithJobs {
        group,
        jobs: jobs_with_preview,
        candidates: Some(candidates),
    })
}

#[tauri::command]
pub async fn update_group(id: i64, updates: Value) -> Result<GroupWithJobs, String> {
    let _group = db::get_group_by_id(id).ok_or("Group not found")?;
    let now = db::now_iso();

    let obj = updates.as_object().ok_or("Expected JSON object")?.clone();

    // Determine if we need to fetch episode titles after the sync work
    let mut fetch_episodes_for: Option<i64> = None;

    // All synchronous DB work in a block so MutexGuard is dropped before any .await
    {
        let allowed = [
            "status",
            "mediaType",
            "parsedTitle",
            "parsedYear",
            "tmdbId",
            "tmdbTitle",
            "tmdbYear",
            "tmdbPosterPath",
            "matchConfidence",
            "destinationId",
        ];

        let field_map: HashMap<&str, &str> = [
            ("status", "status"),
            ("mediaType", "media_type"),
            ("parsedTitle", "parsed_title"),
            ("parsedYear", "parsed_year"),
            ("tmdbId", "tmdb_id"),
            ("tmdbTitle", "tmdb_title"),
            ("tmdbYear", "tmdb_year"),
            ("tmdbPosterPath", "tmdb_poster_path"),
            ("matchConfidence", "match_confidence"),
            ("destinationId", "destination_id"),
        ]
        .into_iter()
        .collect();

        let mut set_parts = vec!["updated_at = ?1".to_string()];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];

        for key in &allowed {
            if let Some(val) = obj.get(*key) {
                let col = field_map[key];
                param_values.push(json_to_sql_param(val));
                set_parts.push(format!("{} = ?{}", col, param_values.len()));
            }
        }

        param_values.push(Box::new(id));
        let sql = format!(
            "UPDATE groups SET {} WHERE id = ?{}",
            set_parts.join(", "),
            param_values.len()
        );

        let db = db::conn();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        db.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;

        // Cascade status changes to child jobs
        if obj.contains_key("status") {
            let status = obj["status"].as_str().unwrap_or("scanned");
            let _ = db.execute(
                "UPDATE jobs SET status = ?1, updated_at = ?2 WHERE group_id = ?3",
                params![status, now, id],
            );
        }

        // Cascade TMDB info to child jobs
        if obj.contains_key("tmdbId") {
            let tmdb_id = obj.get("tmdbId").and_then(|v| v.as_i64());
            let tmdb_title = obj.get("tmdbTitle").and_then(|v| v.as_str());
            let tmdb_year = obj.get("tmdbYear").and_then(|v| v.as_i64());
            let tmdb_poster = obj.get("tmdbPosterPath").and_then(|v| v.as_str());
            let confidence = obj.get("matchConfidence").and_then(|v| v.as_f64());

            let _ = db.execute(
                "UPDATE jobs SET tmdb_id = ?1, tmdb_title = ?2, tmdb_year = ?3, tmdb_poster_path = ?4, match_confidence = ?5, updated_at = ?6 WHERE group_id = ?7",
                params![tmdb_id, tmdb_title, tmdb_year, tmdb_poster, confidence, now, id],
            );

            // Check if we need to fetch episode titles
            if let Some(tid) = tmdb_id {
                let updated_group = db::get_group_by_id(id);
                if let Some(g) = updated_group {
                    let media = obj
                        .get("mediaType")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&g.media_type);
                    if media == "tv" {
                        fetch_episodes_for = Some(tid);
                    }
                }
            }
        }
    } // MutexGuard and param_values dropped here

    // Now safe to .await since no non-Send types are alive
    if let Some(tid) = fetch_episodes_for {
        matcher::fetch_episode_titles(id, tid).await;
    }

    get_group(id).await
}

#[tauri::command]
pub async fn delete_group(id: i64) -> Result<Value, String> {
    let db = db::conn();
    db.execute("DELETE FROM groups WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true }))
}

// ── Jobs ────────────────────────────────────────────────

#[tauri::command]
pub async fn get_jobs(
    status: Option<String>,
    media_type: Option<String>,
    search: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<Value, String> {
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(50);
    let sort_by = sort_by.unwrap_or_else(|| "created_at".into());
    let sort_dir = sort_dir.unwrap_or_else(|| "desc".into());

    let sort_column = match sort_by.as_str() {
        "fileName" => "file_name",
        "fileSize" => "file_size",
        "status" => "status",
        "mediaType" => "media_type",
        "matchConfidence" => "match_confidence",
        _ => "created_at",
    };
    let order = if sort_dir == "asc" { "ASC" } else { "DESC" };

    let db = db::conn();

    let mut conditions = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref s) = status {
        param_values.push(Box::new(s.clone()));
        conditions.push(format!("status = ?{}", param_values.len()));
    }
    if let Some(ref mt) = media_type {
        param_values.push(Box::new(mt.clone()));
        conditions.push(format!("media_type = ?{}", param_values.len()));
    }
    if let Some(ref s) = search {
        param_values.push(Box::new(format!("%{}%", s)));
        conditions.push(format!("file_name LIKE ?{}", param_values.len()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT count(*) FROM jobs {}", where_clause);
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let total: i64 = db
        .query_row(&count_sql, param_refs.as_slice(), |row| row.get(0))
        .unwrap_or(0);

    let offset = (page - 1) * limit;
    param_values.push(Box::new(limit));
    param_values.push(Box::new(offset));
    let select_sql = format!(
        "SELECT * FROM jobs {} ORDER BY {} {} LIMIT ?{} OFFSET ?{}",
        where_clause,
        sort_column,
        order,
        param_values.len() - 1,
        param_values.len(),
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.prepare(&select_sql).map_err(|e| e.to_string())?;
    let jobs: Vec<Job> = stmt
        .query_map(param_refs.as_slice(), db::row_to_job)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({
        "jobs": jobs,
        "total": total,
        "page": page,
        "limit": limit,
    }))
}

#[tauri::command]
pub async fn get_job(id: i64) -> Result<Value, String> {
    let db = db::conn();
    let job = db
        .query_row("SELECT * FROM jobs WHERE id = ?1", params![id], db::row_to_job)
        .map_err(|_| "Job not found".to_string())?;

    let candidates = db::get_candidates_for_group(job.group_id.unwrap_or(-1));
    let mut val = serde_json::to_value(&job).map_err(|e| e.to_string())?;
    val["candidates"] = serde_json::to_value(&candidates).unwrap_or_default();
    Ok(val)
}

#[tauri::command]
pub async fn update_job(id: i64, updates: Value) -> Result<Job, String> {
    let now = db::now_iso();

    let allowed = [
        "status",
        "mediaType",
        "parsedTitle",
        "parsedYear",
        "parsedSeason",
        "parsedEpisode",
        "parsedQuality",
        "parsedCodec",
        "tmdbId",
        "tmdbTitle",
        "tmdbYear",
        "tmdbPosterPath",
        "tmdbEpisodeTitle",
        "matchConfidence",
        "destinationId",
        "destinationPath",
        "transferProgress",
        "transferError",
        "fileCategory",
    ];

    let field_map: HashMap<&str, &str> = [
        ("status", "status"),
        ("mediaType", "media_type"),
        ("parsedTitle", "parsed_title"),
        ("parsedYear", "parsed_year"),
        ("parsedSeason", "parsed_season"),
        ("parsedEpisode", "parsed_episode"),
        ("parsedQuality", "parsed_quality"),
        ("parsedCodec", "parsed_codec"),
        ("tmdbId", "tmdb_id"),
        ("tmdbTitle", "tmdb_title"),
        ("tmdbYear", "tmdb_year"),
        ("tmdbPosterPath", "tmdb_poster_path"),
        ("tmdbEpisodeTitle", "tmdb_episode_title"),
        ("matchConfidence", "match_confidence"),
        ("destinationId", "destination_id"),
        ("destinationPath", "destination_path"),
        ("transferProgress", "transfer_progress"),
        ("transferError", "transfer_error"),
        ("fileCategory", "file_category"),
    ]
    .into_iter()
    .collect();

    let mut set_parts = vec!["updated_at = ?1".to_string()];
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];

    let obj = updates.as_object().ok_or("Expected JSON object")?;
    for key in &allowed {
        if let Some(val) = obj.get(*key) {
            let col = field_map[key];
            param_values.push(json_to_sql_param(val));
            set_parts.push(format!("{} = ?{}", col, param_values.len()));
        }
    }

    param_values.push(Box::new(id));
    let sql = format!(
        "UPDATE jobs SET {} WHERE id = ?{}",
        set_parts.join(", "),
        param_values.len()
    );

    let db = db::conn();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    db.execute(&sql, param_refs.as_slice())
        .map_err(|e| e.to_string())?;

    db.query_row("SELECT * FROM jobs WHERE id = ?1", params![id], db::row_to_job)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_job(id: i64) -> Result<Value, String> {
    let db = db::conn();
    let _ = db.execute(
        "DELETE FROM match_candidates WHERE job_id = ?1",
        params![id],
    );
    db.execute("DELETE FROM jobs WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true }))
}

// ── Bulk actions ────────────────────────────────────────

#[tauri::command]
pub async fn bulk_action(
    action: String,
    job_ids: Option<Vec<i64>>,
    group_ids: Option<Vec<i64>>,
) -> Result<Value, String> {
    let now = db::now_iso();
    let mut affected = 0i64;

    // Handle group-level actions
    if let Some(ref gids) = group_ids {
        if !gids.is_empty() {
            let db = db::conn();
            let placeholders: String = gids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

            match action.as_str() {
                "confirm" => {
                    let sql = format!(
                        "UPDATE groups SET status = 'confirmed', updated_at = ?1 WHERE id IN ({})",
                        placeholders
                    );
                    execute_with_id_params(&db, &sql, &now, gids)?;
                    for gid in gids {
                        let _ = db.execute(
                            "UPDATE jobs SET status = 'confirmed', updated_at = ?1 WHERE group_id = ?2",
                            params![now, gid],
                        );
                    }
                }
                "skip" => {
                    let sql = format!(
                        "UPDATE groups SET status = 'skipped', updated_at = ?1 WHERE id IN ({})",
                        placeholders
                    );
                    execute_with_id_params(&db, &sql, &now, gids)?;
                    for gid in gids {
                        let _ = db.execute(
                            "UPDATE jobs SET status = 'skipped', updated_at = ?1 WHERE group_id = ?2",
                            params![now, gid],
                        );
                    }
                }
                "delete" => {
                    let sql = format!("DELETE FROM groups WHERE id IN ({})", placeholders);
                    execute_with_ids_only(&db, &sql, gids)?;
                }
                "rematch" => {
                    let sql = format!(
                        "UPDATE groups SET status = 'scanned', tmdb_id = NULL, tmdb_title = NULL, tmdb_year = NULL, tmdb_poster_path = NULL, match_confidence = NULL, updated_at = ?1 WHERE id IN ({})",
                        placeholders
                    );
                    execute_with_id_params(&db, &sql, &now, gids)?;
                    for gid in gids {
                        let _ = db.execute(
                            "UPDATE jobs SET status = 'scanned', tmdb_episode_title = NULL, updated_at = ?1 WHERE group_id = ?2",
                            params![now, gid],
                        );
                        let _ = db.execute(
                            "DELETE FROM match_candidates WHERE group_id = ?1",
                            params![gid],
                        );
                    }
                }
                _ => return Err(format!("Unknown action: {}", action)),
            }
            affected += gids.len() as i64;
        }
    }

    // Handle job-level actions
    if let Some(ref jids) = job_ids {
        if !jids.is_empty() {
            let db = db::conn();
            let placeholders: String = jids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

            match action.as_str() {
                "confirm" => {
                    let sql = format!(
                        "UPDATE jobs SET status = 'confirmed', updated_at = ?1 WHERE id IN ({})",
                        placeholders
                    );
                    execute_with_id_params(&db, &sql, &now, jids)?;
                }
                "skip" => {
                    let sql = format!(
                        "UPDATE jobs SET status = 'skipped', updated_at = ?1 WHERE id IN ({})",
                        placeholders
                    );
                    execute_with_id_params(&db, &sql, &now, jids)?;
                }
                "delete" => {
                    for jid in jids {
                        let _ = db.execute(
                            "DELETE FROM match_candidates WHERE job_id = ?1",
                            params![jid],
                        );
                    }
                    let sql = format!("DELETE FROM jobs WHERE id IN ({})", placeholders);
                    execute_with_ids_only(&db, &sql, jids)?;
                }
                "rematch" => {
                    let sql = format!(
                        "UPDATE jobs SET status = 'scanned', tmdb_id = NULL, tmdb_title = NULL, tmdb_year = NULL, tmdb_poster_path = NULL, tmdb_episode_title = NULL, match_confidence = NULL, updated_at = ?1 WHERE id IN ({})",
                        placeholders
                    );
                    execute_with_id_params(&db, &sql, &now, jids)?;
                    for jid in jids {
                        let _ = db.execute(
                            "DELETE FROM match_candidates WHERE job_id = ?1",
                            params![jid],
                        );
                    }
                }
                _ => return Err(format!("Unknown action: {}", action)),
            }
            affected += jids.len() as i64;
        }
    }

    Ok(serde_json::json!({ "success": true, "affected": affected }))
}

// ── Seasons ─────────────────────────────────────────────

#[tauri::command]
pub async fn get_seasons(group_id: i64) -> Result<Value, String> {
    let group = db::get_group_by_id(group_id).ok_or("Group not found")?;
    let tmdb_id = group.tmdb_id.ok_or("Group has no TMDB match")?;

    let seasons = tmdb::get_show_seasons(tmdb_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "seasons": seasons }))
}

#[tauri::command]
pub async fn get_season_episodes(group_id: i64, season: i64) -> Result<Value, String> {
    let group = db::get_group_by_id(group_id).ok_or("Group not found")?;
    let tmdb_id = group.tmdb_id.ok_or("Group has no TMDB match")?;

    let detail = tmdb::get_season(tmdb_id, season)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Season not found")?;

    Ok(serde_json::to_value(&detail).map_err(|e| e.to_string())?)
}

// ── Search TMDB ─────────────────────────────────────────

#[tauri::command]
pub async fn search_tmdb(
    query: String,
    media_type: Option<String>,
    year: Option<i64>,
) -> Result<Value, String> {
    let results = match media_type.as_deref() {
        Some("movie") => tmdb::search_movies(&query, year).await,
        Some("tv") => tmdb::search_tv(&query, year).await,
        _ => tmdb::search_multi(&query, year).await,
    }
    .map_err(|e| e.to_string())?;

    let normalized: Vec<Value> = results
        .iter()
        .take(10)
        .map(|r| {
            let title = r.title.as_deref().or(r.name.as_deref()).unwrap_or("");
            let year: Option<i64> = r
                .release_date
                .as_deref()
                .or(r.first_air_date.as_deref())
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse().ok());
            let overview = r
                .overview
                .as_ref()
                .map(|o| if o.len() > 500 { &o[..500] } else { o.as_str() });

            serde_json::json!({
                "tmdbId": r.id,
                "mediaType": r.media_type.as_deref().unwrap_or("movie"),
                "title": title,
                "year": year,
                "posterPath": r.poster_path,
                "overview": overview,
                "confidence": 1.0,
            })
        })
        .collect();

    Ok(serde_json::json!({ "results": normalized }))
}

// ── Settings ────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings() -> Result<HashMap<String, String>, String> {
    Ok(db::get_all_settings())
}

#[tauri::command]
pub async fn update_settings(updates: HashMap<String, String>) -> Result<HashMap<String, String>, String> {
    let db = db::conn();
    for (key, value) in &updates {
        let _ = db.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        );
    }
    drop(db);
    Ok(db::get_all_settings())
}

// ── Destinations ────────────────────────────────────────

#[tauri::command]
pub async fn get_destinations() -> Result<Vec<Destination>, String> {
    let db = db::conn();
    let mut stmt = db
        .prepare("SELECT * FROM destinations")
        .map_err(|e| e.to_string())?;
    let dests = stmt
        .query_map([], db::row_to_destination)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(dests)
}

#[tauri::command]
pub async fn create_destination(input: CreateDestinationInput) -> Result<Destination, String> {
    let db = db::conn();
    db.execute(
        "INSERT INTO destinations (name, type, base_path, ssh_host, ssh_port, ssh_user, ssh_key_path, ssh_key_passphrase, movie_template, tv_template)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            input.name,
            input.dest_type.as_deref().unwrap_or("local"),
            input.base_path,
            input.ssh_host,
            input.ssh_port.unwrap_or(22),
            input.ssh_user,
            input.ssh_key_path,
            input.ssh_key_passphrase,
            input.movie_template,
            input.tv_template,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = db.last_insert_rowid();
    db::get_destination_by_id(id).ok_or("Failed to create destination".into())
}

#[tauri::command]
pub async fn update_destination(id: i64, input: UpdateDestinationInput) -> Result<Destination, String> {
    let mut set_parts = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    macro_rules! add_field {
        ($field:expr, $col:expr) => {
            if let Some(ref val) = $field {
                param_values.push(Box::new(val.clone()));
                set_parts.push(format!("{} = ?{}", $col, param_values.len()));
            }
        };
    }

    add_field!(input.name, "name");
    add_field!(input.dest_type, "type");
    add_field!(input.base_path, "base_path");
    add_field!(input.ssh_host, "ssh_host");
    if let Some(port) = input.ssh_port {
        param_values.push(Box::new(port));
        set_parts.push(format!("ssh_port = ?{}", param_values.len()));
    }
    add_field!(input.ssh_user, "ssh_user");
    add_field!(input.ssh_key_path, "ssh_key_path");
    add_field!(input.ssh_key_passphrase, "ssh_key_passphrase");
    add_field!(input.movie_template, "movie_template");
    add_field!(input.tv_template, "tv_template");

    if set_parts.is_empty() {
        return db::get_destination_by_id(id).ok_or("Destination not found".into());
    }

    param_values.push(Box::new(id));
    let sql = format!(
        "UPDATE destinations SET {} WHERE id = ?{}",
        set_parts.join(", "),
        param_values.len()
    );

    let db = db::conn();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    db.execute(&sql, param_refs.as_slice())
        .map_err(|e| e.to_string())?;

    drop(db);
    db::get_destination_by_id(id).ok_or("Destination not found".into())
}

#[tauri::command]
pub async fn delete_destination(id: i64) -> Result<Value, String> {
    let db = db::conn();
    db.execute("DELETE FROM destinations WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true }))
}

// ── Test SSH Connection ─────────────────────────────────

#[tauri::command]
pub async fn test_ssh_connection(input: TestSshInput) -> Result<Value, String> {
    let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let addr = format!("{}:{}", input.ssh_host, input.ssh_port.unwrap_or(22));
        let tcp = std::net::TcpStream::connect_timeout(
            &addr.parse().map_err(|e: std::net::AddrParseError| e.to_string())?,
            std::time::Duration::from_secs(10),
        )
        .map_err(|e| format!("Connection failed: {}", e))?;

        let mut sess = ssh2::Session::new().map_err(|e| e.to_string())?;
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| format!("Handshake failed: {}", e))?;

        if let Some(ref key_path) = input.ssh_key_path {
            let passphrase = input.ssh_key_passphrase.as_deref();
            sess.userauth_pubkey_file(
                &input.ssh_user,
                None,
                std::path::Path::new(key_path),
                passphrase,
            )
            .map_err(|e| format!("Auth failed: {}", e))?;
        } else {
            sess.userauth_agent(&input.ssh_user)
                .map_err(|e| format!("Agent auth failed: {}", e))?;
        }

        if let Some(ref base_path) = input.base_path {
            let sftp = sess.sftp().map_err(|e| format!("SFTP error: {}", e))?;
            sftp.stat(std::path::Path::new(base_path))
                .map_err(|_| format!("Base path not found on remote: {}", base_path))?;
        }

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?;

    match result {
        Ok(()) => Ok(serde_json::json!({ "ok": true })),
        Err(e) => Ok(serde_json::json!({ "ok": false, "error": e })),
    }
}

// ── Transfer ────────────────────────────────────────────

#[tauri::command]
pub async fn start_transfer(
    app: AppHandle,
    job_ids: Option<Vec<i64>>,
    group_ids: Option<Vec<i64>>,
    destination_id: i64,
) -> Result<Value, String> {
    let mut all_job_ids: HashSet<i64> = job_ids.unwrap_or_default().into_iter().collect();

    // Expand group IDs to their confirmed child jobs
    if let Some(gids) = group_ids {
        for gid in gids {
            let jobs = db::get_jobs_for_group(gid);
            for j in jobs {
                if j.status == "confirmed" {
                    all_job_ids.insert(j.id);
                }
            }
        }
    }

    if all_job_ids.is_empty() {
        return Err("No confirmed jobs to transfer".into());
    }

    let ids: Vec<i64> = all_job_ids.into_iter().collect();
    let count = transfer::queue_transfers(app, ids, destination_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({ "queued": count }))
}

// ── Helpers ─────────────────────────────────────────────

fn json_to_sql_param(val: &Value) -> Box<dyn rusqlite::types::ToSql> {
    match val {
        Value::Null => Box::new(None::<String>),
        Value::Bool(b) => Box::new(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        Value::String(s) => Box::new(s.clone()),
        _ => Box::new(val.to_string()),
    }
}

fn execute_with_id_params(
    db: &rusqlite::Connection,
    sql: &str,
    now: &str,
    ids: &[i64],
) -> Result<(), String> {
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    param_values.push(Box::new(now.to_string()));
    for id in ids {
        param_values.push(Box::new(*id));
    }
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    db.execute(sql, param_refs.as_slice())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn execute_with_ids_only(
    db: &rusqlite::Connection,
    sql: &str,
    ids: &[i64],
) -> Result<(), String> {
    let param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
        ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>).collect();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    db.execute(sql, param_refs.as_slice())
        .map_err(|e| e.to_string())?;
    Ok(())
}
