//! Local SQLite persistence.
//!
//! Haleacas is offline-first and local-only by default (spec sections 28-29):
//! we never store file contents, never store secrets, and everything here
//! lives on disk under the app's local data directory.

pub mod ignored_findings;
pub mod repositories;
pub mod settings;

use rusqlite::Connection;
use std::path::PathBuf;

/// Resolve (and create, if missing) the directory Haleacas stores its local
/// database in, using OS-appropriate conventions via `directories`.
pub fn data_dir() -> anyhow::Result<PathBuf> {
    let proj = directories::ProjectDirs::from("dev", "haleacas", "Haleacas")
        .ok_or_else(|| anyhow::anyhow!("could not resolve a local app data directory"))?;
    let dir = proj.data_dir().to_path_buf();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn open() -> anyhow::Result<Connection> {
    let dir = data_dir()?;
    let db_path = dir.join("haleacas.sqlite3");
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Forward-only schema migrations, tracked by `PRAGMA user_version`.
///
/// Design note (spec section 28: "design the schema for migration"): each
/// migration is a numbered step applied exactly once. Add new steps by
/// appending to this match rather than editing earlier ones.
fn migrate(conn: &Connection) -> anyhow::Result<()> {
    let mut version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if version < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS repositories (
                id            TEXT PRIMARY KEY,
                path          TEXT NOT NULL UNIQUE,
                name          TEXT NOT NULL,
                fingerprint   TEXT NOT NULL,
                last_scanned  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_repositories_path ON repositories(path);
            "#,
        )?;
        version = 1;
        conn.pragma_update(None, "user_version", version)?;
    }

    if version < 2 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS ignored_findings (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_path TEXT NOT NULL,
                file      TEXT NOT NULL,
                line      INTEGER NOT NULL,
                reason    TEXT NOT NULL,
                UNIQUE(repo_path, file, line, reason)
            );

            CREATE INDEX IF NOT EXISTS idx_ignored_findings_repo
                ON ignored_findings(repo_path);
            "#,
        )?;
        version = 2;
        conn.pragma_update(None, "user_version", version)?;
    }

    Ok(())
}
