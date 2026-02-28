use anyhow::Result;
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, Row};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tracing::info;

use crate::models::*;

static DB: OnceCell<Mutex<Connection>> = OnceCell::new();

/// Initialize the database, creating tables and default settings.
pub fn initialize(app: &AppHandle) -> Result<()> {
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("data"));
    fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("reelname.db");
    info!("Database path: {}", db_path.display());

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

    create_tables(&conn)?;
    run_migrations(&conn);
    insert_default_settings(&conn)?;

    DB.set(Mutex::new(conn))
        .map_err(|_| anyhow::anyhow!("Database already initialized"))?;

    Ok(())
}

/// Get a reference to the database connection.
pub fn conn() -> std::sync::MutexGuard<'static, Connection> {
    DB.get().expect("Database not initialized").lock().unwrap()
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            status TEXT NOT NULL DEFAULT 'scanned',
            media_type TEXT NOT NULL DEFAULT 'unknown',
            folder_path TEXT NOT NULL,
            folder_name TEXT NOT NULL,
            total_file_count INTEGER NOT NULL DEFAULT 0,
            total_file_size INTEGER NOT NULL DEFAULT 0,
            parsed_title TEXT,
            parsed_year INTEGER,
            tmdb_id INTEGER,
            tmdb_title TEXT,
            tmdb_year INTEGER,
            tmdb_poster_path TEXT,
            match_confidence REAL,
            destination_id INTEGER REFERENCES destinations(id),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id INTEGER REFERENCES groups(id) ON DELETE CASCADE,
            status TEXT NOT NULL DEFAULT 'scanned',
            media_type TEXT NOT NULL DEFAULT 'unknown',
            file_category TEXT NOT NULL DEFAULT 'episode',
            extra_type TEXT,
            source_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_extension TEXT NOT NULL,
            parsed_title TEXT,
            parsed_year INTEGER,
            parsed_season INTEGER,
            parsed_episode INTEGER,
            parsed_quality TEXT,
            parsed_codec TEXT,
            tmdb_id INTEGER,
            tmdb_title TEXT,
            tmdb_year INTEGER,
            tmdb_poster_path TEXT,
            tmdb_episode_title TEXT,
            match_confidence REAL,
            destination_id INTEGER REFERENCES destinations(id),
            destination_path TEXT,
            transfer_progress REAL,
            transfer_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS match_candidates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id INTEGER REFERENCES jobs(id) ON DELETE CASCADE,
            group_id INTEGER REFERENCES groups(id) ON DELETE CASCADE,
            tmdb_id INTEGER NOT NULL,
            media_type TEXT NOT NULL,
            title TEXT NOT NULL,
            year INTEGER,
            poster_path TEXT,
            overview TEXT,
            confidence REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS destinations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            type TEXT NOT NULL DEFAULT 'local',
            base_path TEXT NOT NULL,
            ssh_host TEXT,
            ssh_port INTEGER DEFAULT 22,
            ssh_user TEXT,
            ssh_key_path TEXT,
            ssh_key_passphrase TEXT,
            movie_template TEXT,
            tv_template TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

fn try_exec(conn: &Connection, sql: &str) {
    let _ = conn.execute_batch(sql);
}

fn run_migrations(conn: &Connection) {
    // These are safe to run multiple times — they'll fail silently if column exists
    try_exec(
        conn,
        "ALTER TABLE destinations ADD COLUMN ssh_key_passphrase TEXT",
    );
    try_exec(
        conn,
        "ALTER TABLE jobs ADD COLUMN group_id INTEGER REFERENCES groups(id) ON DELETE CASCADE",
    );
    try_exec(
        conn,
        "ALTER TABLE jobs ADD COLUMN file_category TEXT NOT NULL DEFAULT 'episode'",
    );
    try_exec(conn, "ALTER TABLE jobs ADD COLUMN extra_type TEXT");
    try_exec(
        conn,
        "ALTER TABLE match_candidates ADD COLUMN group_id INTEGER REFERENCES groups(id) ON DELETE CASCADE",
    );
}

fn insert_default_settings(conn: &Connection) -> Result<()> {
    let defaults = [
        ("scan_path", ""),
        ("tmdb_api_key", ""),
        ("auto_match_threshold", "0.85"),
        ("naming_preset", "jellyfin"),
        ("specials_folder_name", "Specials"),
        ("extras_folder_name", "Extras"),
    ];

    let mut stmt =
        conn.prepare("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")?;
    for (key, value) in &defaults {
        stmt.execute(params![key, value])?;
    }
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn get_setting(key: &str) -> Option<String> {
    let db = conn();
    db.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn get_all_settings() -> std::collections::HashMap<String, String> {
    let db = conn();
    let mut stmt = db.prepare("SELECT key, value FROM settings").unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap();
    rows.filter_map(|r| r.ok()).collect()
}

// ── Row mappers ─────────────────────────────────────────

pub fn row_to_group(row: &Row) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get("id")?,
        status: row.get("status")?,
        media_type: row.get("media_type")?,
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

pub fn row_to_job(row: &Row) -> rusqlite::Result<Job> {
    Ok(Job {
        id: row.get("id")?,
        group_id: row.get("group_id")?,
        status: row.get("status")?,
        media_type: row.get("media_type")?,
        file_category: row.get("file_category")?,
        extra_type: row.get("extra_type")?,
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

pub fn row_to_candidate(row: &Row) -> rusqlite::Result<MatchCandidate> {
    Ok(MatchCandidate {
        id: row.get("id")?,
        job_id: row.get("job_id")?,
        group_id: row.get("group_id")?,
        tmdb_id: row.get("tmdb_id")?,
        media_type: row.get("media_type")?,
        title: row.get("title")?,
        year: row.get("year")?,
        poster_path: row.get("poster_path")?,
        overview: row.get("overview")?,
        confidence: row.get("confidence")?,
    })
}

pub fn row_to_destination(row: &Row) -> rusqlite::Result<Destination> {
    Ok(Destination {
        id: row.get("id")?,
        name: row.get("name")?,
        dest_type: row.get("type")?,
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

pub fn get_group_by_id(id: i64) -> Option<Group> {
    let db = conn();
    db.query_row("SELECT * FROM groups WHERE id = ?1", params![id], row_to_group)
        .ok()
}

pub fn get_jobs_for_group(group_id: i64) -> Vec<Job> {
    let db = conn();
    let mut stmt = db
        .prepare("SELECT * FROM jobs WHERE group_id = ?1")
        .unwrap();
    stmt.query_map(params![group_id], row_to_job)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn get_candidates_for_group(group_id: i64) -> Vec<MatchCandidate> {
    let db = conn();
    let mut stmt = db
        .prepare("SELECT * FROM match_candidates WHERE group_id = ?1")
        .unwrap();
    stmt.query_map(params![group_id], row_to_candidate)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn get_destination_by_id(id: i64) -> Option<Destination> {
    let db = conn();
    db.query_row(
        "SELECT * FROM destinations WHERE id = ?1",
        params![id],
        row_to_destination,
    )
    .ok()
}
