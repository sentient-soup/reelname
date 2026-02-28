pub mod queries;
pub mod schema;

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

pub type DbConn = Arc<Mutex<Connection>>;

/// Get the database directory path.
/// Uses REELNAME_DATA_DIR env var, or falls back to ./data/
pub fn db_path() -> PathBuf {
    if let Ok(dir) = std::env::var("REELNAME_DATA_DIR") {
        PathBuf::from(dir).join("reelname.db")
    } else {
        PathBuf::from("data").join("reelname.db")
    }
}

/// Open (or create) the database and run initialization.
pub fn open_database(path: &Path) -> Result<DbConn, rusqlite::Error> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(path)?;

    // Set pragmas
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;",
    )?;

    initialize_database(&conn)?;

    info!("Database opened at {}", path.display());
    Ok(Arc::new(Mutex::new(conn)))
}

/// Silently execute SQL, ignoring errors (for idempotent migrations).
fn try_exec(conn: &Connection, sql: &str) {
    if let Err(e) = conn.execute_batch(sql) {
        warn!("Migration (ignored): {} — {}", sql.chars().take(80).collect::<String>(), e);
    }
}

/// Create all tables and run migrations.
fn initialize_database(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS groups (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            status          TEXT NOT NULL DEFAULT 'scanned',
            media_type      TEXT NOT NULL DEFAULT 'unknown',
            folder_path     TEXT NOT NULL,
            folder_name     TEXT NOT NULL,
            total_file_count INTEGER NOT NULL DEFAULT 0,
            total_file_size  INTEGER NOT NULL DEFAULT 0,
            parsed_title    TEXT,
            parsed_year     INTEGER,
            tmdb_id         INTEGER,
            tmdb_title      TEXT,
            tmdb_year       INTEGER,
            tmdb_poster_path TEXT,
            match_confidence REAL,
            destination_id  INTEGER REFERENCES destinations(id),
            created_at      TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS jobs (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id        INTEGER REFERENCES groups(id) ON DELETE CASCADE,
            status          TEXT NOT NULL DEFAULT 'scanned',
            media_type      TEXT NOT NULL DEFAULT 'unknown',
            file_category   TEXT NOT NULL DEFAULT 'episode',
            extra_type      TEXT,
            source_path     TEXT NOT NULL,
            file_name       TEXT NOT NULL,
            file_size       INTEGER NOT NULL,
            file_extension  TEXT NOT NULL,
            parsed_title    TEXT,
            parsed_year     INTEGER,
            parsed_season   INTEGER,
            parsed_episode  INTEGER,
            parsed_quality  TEXT,
            parsed_codec    TEXT,
            tmdb_id         INTEGER,
            tmdb_title      TEXT,
            tmdb_year       INTEGER,
            tmdb_poster_path TEXT,
            tmdb_episode_title TEXT,
            match_confidence REAL,
            destination_id  INTEGER REFERENCES destinations(id),
            destination_path TEXT,
            transfer_progress REAL,
            transfer_error  TEXT,
            created_at      TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS match_candidates (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id          INTEGER REFERENCES jobs(id) ON DELETE CASCADE,
            group_id        INTEGER REFERENCES groups(id) ON DELETE CASCADE,
            tmdb_id         INTEGER NOT NULL,
            media_type      TEXT NOT NULL,
            title           TEXT NOT NULL,
            year            INTEGER,
            poster_path     TEXT,
            overview        TEXT,
            confidence      REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS destinations (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            type            TEXT NOT NULL DEFAULT 'local',
            base_path       TEXT NOT NULL,
            ssh_host        TEXT,
            ssh_port        INTEGER DEFAULT 22,
            ssh_user        TEXT,
            ssh_key_path    TEXT,
            ssh_key_passphrase TEXT,
            movie_template  TEXT,
            tv_template     TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key             TEXT PRIMARY KEY,
            value           TEXT NOT NULL
        );"
    )?;

    // ── Migrations (idempotent) ──
    // Add new ALTER TABLE migrations here as the schema evolves.
    // try_exec(conn, "ALTER TABLE groups ADD COLUMN new_col TEXT");

    // ── Default settings ──
    let defaults = [
        ("scan_path", ""),
        ("tmdb_api_key", ""),
        ("auto_match_threshold", "0.85"),
        ("naming_preset", "jellyfin"),
        ("specials_folder_name", "Specials"),
        ("extras_folder_name", "Extras"),
    ];

    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
    )?;
    for (key, value) in &defaults {
        stmt.execute(rusqlite::params![key, value])?;
    }

    info!("Database initialized");
    Ok(())
}
