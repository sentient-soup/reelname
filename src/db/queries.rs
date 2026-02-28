use super::DbConn;
use super::schema::*;
use rusqlite::{params, Row};

// ── Row mapping helpers ──

fn row_to_group(row: &Row<'_>) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get("id")?,
        status: GroupStatus::from_str(row.get::<_, String>("status")?.as_str()),
        media_type: MediaType::from_str(row.get::<_, String>("media_type")?.as_str()),
        folder_path: row.get("folder_path")?,
        folder_name: row.get("folder_name")?,
        total_file_count: row.get("total_file_count")?,
        total_file_size: row.get("total_file_size")?,
        parsed_title: row.get("parsed_title")?,
        parsed_year: row.get("parsed_year")?,
        tmdb_id: row.get("tmdb_id")?,
        tmdb_title: row.get("tmdb_title")?,
        tmdb_year: row.get("tmdb_year")?,
        tmdb_poster_path: row.get("tmdb_poster_path")?,
        match_confidence: row.get("match_confidence")?,
        destination_id: row.get("destination_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_job(row: &Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: row.get("id")?,
        group_id: row.get("group_id")?,
        status: GroupStatus::from_str(row.get::<_, String>("status")?.as_str()),
        media_type: MediaType::from_str(row.get::<_, String>("media_type")?.as_str()),
        file_category: FileCategory::from_str(row.get::<_, String>("file_category")?.as_str()),
        extra_type: row
            .get::<_, Option<String>>("extra_type")?
            .map(|s| ExtraType::from_str(&s)),
        source_path: row.get("source_path")?,
        file_name: row.get("file_name")?,
        file_size: row.get("file_size")?,
        file_extension: row.get("file_extension")?,
        parsed_title: row.get("parsed_title")?,
        parsed_year: row.get("parsed_year")?,
        parsed_season: row.get("parsed_season")?,
        parsed_episode: row.get("parsed_episode")?,
        parsed_quality: row.get("parsed_quality")?,
        parsed_codec: row.get("parsed_codec")?,
        tmdb_id: row.get("tmdb_id")?,
        tmdb_title: row.get("tmdb_title")?,
        tmdb_year: row.get("tmdb_year")?,
        tmdb_poster_path: row.get("tmdb_poster_path")?,
        tmdb_episode_title: row.get("tmdb_episode_title")?,
        match_confidence: row.get("match_confidence")?,
        destination_id: row.get("destination_id")?,
        destination_path: row.get("destination_path")?,
        transfer_progress: row.get("transfer_progress")?,
        transfer_error: row.get("transfer_error")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_match_candidate(row: &Row<'_>) -> rusqlite::Result<MatchCandidate> {
    Ok(MatchCandidate {
        id: row.get("id")?,
        job_id: row.get("job_id")?,
        group_id: row.get("group_id")?,
        tmdb_id: row.get("tmdb_id")?,
        media_type: MediaType::from_str(row.get::<_, String>("media_type")?.as_str()),
        title: row.get("title")?,
        year: row.get("year")?,
        poster_path: row.get("poster_path")?,
        overview: row.get("overview")?,
        confidence: row.get("confidence")?,
    })
}

fn row_to_destination(row: &Row<'_>) -> rusqlite::Result<Destination> {
    Ok(Destination {
        id: row.get("id")?,
        name: row.get("name")?,
        dest_type: DestinationType::from_str(row.get::<_, String>("type")?.as_str()),
        base_path: row.get("base_path")?,
        ssh_host: row.get("ssh_host")?,
        ssh_port: row.get("ssh_port")?,
        ssh_user: row.get("ssh_user")?,
        ssh_key_path: row.get("ssh_key_path")?,
        ssh_key_passphrase: row.get("ssh_key_passphrase")?,
        movie_template: row.get("movie_template")?,
        tv_template: row.get("tv_template")?,
    })
}

fn row_to_setting(row: &Row<'_>) -> rusqlite::Result<Setting> {
    Ok(Setting {
        key: row.get("key")?,
        value: row.get("value")?,
    })
}

// ── Groups ──

pub fn insert_group(
    conn: &DbConn,
    folder_path: &str,
    folder_name: &str,
    parsed_title: Option<&str>,
    parsed_year: Option<i64>,
    media_type: MediaType,
    total_file_count: i64,
    total_file_size: i64,
) -> rusqlite::Result<i64> {
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO groups (folder_path, folder_name, parsed_title, parsed_year, media_type, total_file_count, total_file_size)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            folder_path,
            folder_name,
            parsed_title,
            parsed_year,
            media_type.as_str(),
            total_file_count,
            total_file_size,
        ],
    )?;
    Ok(db.last_insert_rowid())
}

pub fn fetch_groups(
    conn: &DbConn,
    status: Option<GroupStatus>,
    media_type: Option<MediaType>,
    search: Option<&str>,
    sort_by: &str,
    sort_dir: &str,
    page: i64,
    per_page: i64,
) -> rusqlite::Result<(Vec<Group>, i64)> {
    let db = conn.lock().unwrap();

    let mut conditions = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = status {
        conditions.push(format!("status = ?{}", param_values.len() + 1));
        param_values.push(Box::new(s.as_str().to_string()));
    }
    if let Some(mt) = media_type {
        conditions.push(format!("media_type = ?{}", param_values.len() + 1));
        param_values.push(Box::new(mt.as_str().to_string()));
    }
    if let Some(q) = search {
        if !q.is_empty() {
            conditions.push(format!(
                "(folder_name LIKE ?{n} OR parsed_title LIKE ?{n} OR tmdb_title LIKE ?{n})",
                n = param_values.len() + 1
            ));
            param_values.push(Box::new(format!("%{q}%")));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Validate sort column
    let sort_col = match sort_by {
        "folderName" | "folder_name" => "folder_name",
        "totalFileSize" | "total_file_size" => "total_file_size",
        "status" => "status",
        "mediaType" | "media_type" => "media_type",
        _ => "created_at",
    };
    let dir = if sort_dir.eq_ignore_ascii_case("asc") {
        "ASC"
    } else {
        "DESC"
    };

    // Count
    let count_sql = format!("SELECT COUNT(*) FROM groups {where_clause}");
    let params_ref: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|b| b.as_ref()).collect();
    let total: i64 = db.query_row(&count_sql, params_ref.as_slice(), |r| r.get(0))?;

    // Query
    let offset = (page - 1) * per_page;
    let query_sql = format!(
        "SELECT * FROM groups {where_clause} ORDER BY {sort_col} {dir} LIMIT ?{n1} OFFSET ?{n2}",
        n1 = param_values.len() + 1,
        n2 = param_values.len() + 2,
    );

    let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = param_values;
    all_params.push(Box::new(per_page));
    all_params.push(Box::new(offset));

    let params_ref: Vec<&dyn rusqlite::types::ToSql> =
        all_params.iter().map(|b| b.as_ref()).collect();

    let mut stmt = db.prepare(&query_sql)?;
    let groups = stmt
        .query_map(params_ref.as_slice(), row_to_group)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok((groups, total))
}

pub fn fetch_group(conn: &DbConn, id: i64) -> rusqlite::Result<Option<Group>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT * FROM groups WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], row_to_group)?;
    Ok(rows.next().transpose()?)
}

pub fn update_group(
    conn: &DbConn,
    id: i64,
    updates: &[(&str, &dyn rusqlite::types::ToSql)],
) -> rusqlite::Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    let db = conn.lock().unwrap();
    let set_clauses: Vec<String> = updates
        .iter()
        .enumerate()
        .map(|(i, (col, _))| format!("{col} = ?{}", i + 1))
        .collect();
    let sql = format!(
        "UPDATE groups SET {}, updated_at = datetime('now') WHERE id = ?{}",
        set_clauses.join(", "),
        updates.len() + 1
    );
    let mut param_values: Vec<&dyn rusqlite::types::ToSql> =
        updates.iter().map(|(_, v)| *v).collect();
    param_values.push(&id);
    db.execute(&sql, param_values.as_slice())?;
    Ok(())
}

pub fn delete_group(conn: &DbConn, id: i64) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    db.execute("DELETE FROM groups WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_all_groups(conn: &DbConn) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    db.execute_batch("DELETE FROM match_candidates; DELETE FROM jobs; DELETE FROM groups;")?;
    Ok(())
}

pub fn group_exists_by_folder(conn: &DbConn, folder_path: &str) -> rusqlite::Result<bool> {
    let db = conn.lock().unwrap();
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM groups WHERE folder_path = ?1",
        params![folder_path],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

// ── Jobs ──

pub fn insert_job(
    conn: &DbConn,
    group_id: i64,
    source_path: &str,
    file_name: &str,
    file_size: i64,
    file_extension: &str,
    media_type: MediaType,
    file_category: FileCategory,
    extra_type: Option<ExtraType>,
    parsed_title: Option<&str>,
    parsed_year: Option<i64>,
    parsed_season: Option<i64>,
    parsed_episode: Option<i64>,
    parsed_quality: Option<&str>,
    parsed_codec: Option<&str>,
) -> rusqlite::Result<i64> {
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO jobs (group_id, source_path, file_name, file_size, file_extension,
         media_type, file_category, extra_type, parsed_title, parsed_year,
         parsed_season, parsed_episode, parsed_quality, parsed_codec)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            group_id,
            source_path,
            file_name,
            file_size,
            file_extension,
            media_type.as_str(),
            file_category.as_str(),
            extra_type.map(|e| e.as_str().to_string()),
            parsed_title,
            parsed_year,
            parsed_season,
            parsed_episode,
            parsed_quality,
            parsed_codec,
        ],
    )?;
    Ok(db.last_insert_rowid())
}

pub fn fetch_jobs_for_group(conn: &DbConn, group_id: i64) -> rusqlite::Result<Vec<Job>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT * FROM jobs WHERE group_id = ?1 ORDER BY file_name")?;
    let jobs = stmt
        .query_map(params![group_id], row_to_job)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(jobs)
}

pub fn fetch_job(conn: &DbConn, id: i64) -> rusqlite::Result<Option<Job>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT * FROM jobs WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], row_to_job)?;
    Ok(rows.next().transpose()?)
}

pub fn update_job(
    conn: &DbConn,
    id: i64,
    updates: &[(&str, &dyn rusqlite::types::ToSql)],
) -> rusqlite::Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    let db = conn.lock().unwrap();
    let set_clauses: Vec<String> = updates
        .iter()
        .enumerate()
        .map(|(i, (col, _))| format!("{col} = ?{}", i + 1))
        .collect();
    let sql = format!(
        "UPDATE jobs SET {}, updated_at = datetime('now') WHERE id = ?{}",
        set_clauses.join(", "),
        updates.len() + 1
    );
    let mut param_values: Vec<&dyn rusqlite::types::ToSql> =
        updates.iter().map(|(_, v)| *v).collect();
    param_values.push(&id);
    db.execute(&sql, param_values.as_slice())?;
    Ok(())
}

pub fn update_jobs_for_group(
    conn: &DbConn,
    group_id: i64,
    updates: &[(&str, &dyn rusqlite::types::ToSql)],
) -> rusqlite::Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    let db = conn.lock().unwrap();
    let set_clauses: Vec<String> = updates
        .iter()
        .enumerate()
        .map(|(i, (col, _))| format!("{col} = ?{}", i + 1))
        .collect();
    let sql = format!(
        "UPDATE jobs SET {}, updated_at = datetime('now') WHERE group_id = ?{}",
        set_clauses.join(", "),
        updates.len() + 1
    );
    let mut param_values: Vec<&dyn rusqlite::types::ToSql> =
        updates.iter().map(|(_, v)| *v).collect();
    param_values.push(&group_id);
    db.execute(&sql, param_values.as_slice())?;
    Ok(())
}

pub fn delete_job(conn: &DbConn, id: i64) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    db.execute("DELETE FROM jobs WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn fetch_scannable_groups(conn: &DbConn) -> rusqlite::Result<Vec<Group>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT * FROM groups WHERE status IN ('scanned', 'ambiguous') ORDER BY id",
    )?;
    let groups = stmt
        .query_map([], row_to_group)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(groups)
}

pub fn fetch_confirmed_jobs(conn: &DbConn, group_ids: &[i64]) -> rusqlite::Result<Vec<Job>> {
    if group_ids.is_empty() {
        return Ok(vec![]);
    }
    let db = conn.lock().unwrap();
    let placeholders: Vec<String> = (1..=group_ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT * FROM jobs WHERE group_id IN ({}) AND status = 'confirmed' ORDER BY id",
        placeholders.join(", ")
    );
    let params: Vec<&dyn rusqlite::types::ToSql> =
        group_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let mut stmt = db.prepare(&sql)?;
    let jobs = stmt
        .query_map(params.as_slice(), row_to_job)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(jobs)
}

// ── Match Candidates ──

pub fn insert_match_candidate(
    conn: &DbConn,
    group_id: i64,
    tmdb_id: i64,
    media_type: MediaType,
    title: &str,
    year: Option<i64>,
    poster_path: Option<&str>,
    overview: Option<&str>,
    confidence: f64,
) -> rusqlite::Result<i64> {
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO match_candidates (group_id, tmdb_id, media_type, title, year, poster_path, overview, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            group_id,
            tmdb_id,
            media_type.as_str(),
            title,
            year,
            poster_path,
            overview,
            confidence,
        ],
    )?;
    Ok(db.last_insert_rowid())
}

pub fn fetch_candidates_for_group(
    conn: &DbConn,
    group_id: i64,
) -> rusqlite::Result<Vec<MatchCandidate>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT * FROM match_candidates WHERE group_id = ?1 ORDER BY confidence DESC",
    )?;
    let candidates = stmt
        .query_map(params![group_id], row_to_match_candidate)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(candidates)
}

pub fn delete_candidates_for_group(conn: &DbConn, group_id: i64) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    db.execute(
        "DELETE FROM match_candidates WHERE group_id = ?1",
        params![group_id],
    )?;
    Ok(())
}

// ── Destinations ──

pub fn fetch_destinations(conn: &DbConn) -> rusqlite::Result<Vec<Destination>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT * FROM destinations ORDER BY name")?;
    let dests = stmt
        .query_map([], row_to_destination)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(dests)
}

pub fn fetch_destination(conn: &DbConn, id: i64) -> rusqlite::Result<Option<Destination>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT * FROM destinations WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], row_to_destination)?;
    Ok(rows.next().transpose()?)
}

pub fn insert_destination(
    conn: &DbConn,
    name: &str,
    dest_type: DestinationType,
    base_path: &str,
    ssh_host: Option<&str>,
    ssh_port: Option<i64>,
    ssh_user: Option<&str>,
    ssh_key_path: Option<&str>,
    ssh_key_passphrase: Option<&str>,
) -> rusqlite::Result<i64> {
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO destinations (name, type, base_path, ssh_host, ssh_port, ssh_user, ssh_key_path, ssh_key_passphrase)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            name,
            dest_type.as_str(),
            base_path,
            ssh_host,
            ssh_port,
            ssh_user,
            ssh_key_path,
            ssh_key_passphrase,
        ],
    )?;
    Ok(db.last_insert_rowid())
}

pub fn delete_destination(conn: &DbConn, id: i64) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    db.execute("DELETE FROM destinations WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Settings ──

pub fn fetch_settings(conn: &DbConn) -> rusqlite::Result<Vec<Setting>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT * FROM settings ORDER BY key")?;
    let settings = stmt
        .query_map([], row_to_setting)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(settings)
}

pub fn get_setting(conn: &DbConn, key: &str) -> rusqlite::Result<Option<String>> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
    Ok(rows.next().transpose()?)
}

pub fn set_setting(conn: &DbConn, key: &str, value: &str) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    db.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )?;
    Ok(())
}

pub fn update_settings(conn: &DbConn, settings: &[(&str, &str)]) -> rusqlite::Result<()> {
    let db = conn.lock().unwrap();
    let mut stmt = db.prepare(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
    )?;
    for (key, value) in settings {
        stmt.execute(params![key, value])?;
    }
    Ok(())
}
