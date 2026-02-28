import Database from "better-sqlite3";
import { drizzle } from "drizzle-orm/better-sqlite3";
import * as schema from "./schema";
import path from "path";
import fs from "fs";

function getDataDir(): string {
  if (process.env.REELNAME_DATA_DIR) {
    return process.env.REELNAME_DATA_DIR;
  }
  return path.join(process.cwd(), "data");
}

export const DATA_DIR = getDataDir();
const DB_PATH = path.join(DATA_DIR, "reelname.db");

// During the Next.js build phase, parallel workers race to open the same
// SQLite file and set journal_mode=WAL, causing SQLITE_BUSY on macOS even
// with busy_timeout set.  Use an in-memory database instead — the build
// workers only import the module for route analysis; handlers never run.
const isBuildPhase = process.env.NEXT_PHASE === "phase-production-build";

// Ensure data directory exists (skipped during build; no real file needed)
if (!isBuildPhase) {
  fs.mkdirSync(DATA_DIR, { recursive: true });
}

const sqlite = new Database(isBuildPhase ? ":memory:" : DB_PATH);
// Set busy_timeout first so subsequent pragmas and initializeDatabase()
// retry instead of immediately throwing SQLITE_BUSY when multiple
// Next.js build workers open the same database in parallel.
sqlite.pragma("busy_timeout = 5000");
sqlite.pragma("journal_mode = WAL");
sqlite.pragma("foreign_keys = ON");

export const db = drizzle(sqlite, { schema });

function tryExec(sql: string) {
  try {
    sqlite.exec(sql);
  } catch {
    // Column/table may already exist
  }
}

function migrateMatchCandidatesNullableJobId() {
  // Check if job_id column has a NOT NULL constraint by trying a null insert
  try {
    const info = sqlite.prepare("PRAGMA table_info(match_candidates)").all() as Array<{
      name: string;
      notnull: number;
    }>;
    const jobIdCol = info.find((c) => c.name === "job_id");
    if (!jobIdCol || jobIdCol.notnull === 0) return; // Already nullable or doesn't exist

    // Recreate table with nullable job_id
    sqlite.exec(`
      ALTER TABLE match_candidates RENAME TO _match_candidates_old;
      CREATE TABLE match_candidates (
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
      INSERT INTO match_candidates SELECT * FROM _match_candidates_old;
      DROP TABLE _match_candidates_old;
    `);
  } catch {
    // Table might not exist yet, that's fine
  }
}

function initializeDatabase() {
  sqlite.exec(`
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
      movie_template TEXT,
      tv_template TEXT
    );

    CREATE TABLE IF NOT EXISTS settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );
  `);

  // Migration: add ssh_key_passphrase to destinations
  tryExec("ALTER TABLE destinations ADD COLUMN ssh_key_passphrase TEXT");

  // Migration: add new columns to existing tables
  tryExec("ALTER TABLE jobs ADD COLUMN group_id INTEGER REFERENCES groups(id) ON DELETE CASCADE");
  tryExec("ALTER TABLE jobs ADD COLUMN file_category TEXT NOT NULL DEFAULT 'episode'");
  tryExec("ALTER TABLE jobs ADD COLUMN extra_type TEXT");
  tryExec("ALTER TABLE match_candidates ADD COLUMN group_id INTEGER REFERENCES groups(id) ON DELETE CASCADE");

  // Migration: recreate match_candidates with nullable job_id
  // (old table had implicit NOT NULL on job_id)
  migrateMatchCandidatesNullableJobId();

  // Insert default settings if not present
  const insertSetting = sqlite.prepare(
    "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)"
  );
  insertSetting.run("scan_path", "");
  insertSetting.run("tmdb_api_key", "");
  insertSetting.run("auto_match_threshold", "0.85");
  insertSetting.run("naming_preset", "jellyfin");
  insertSetting.run("specials_folder_name", "Specials");
  insertSetting.run("extras_folder_name", "Extras");
}

initializeDatabase();
