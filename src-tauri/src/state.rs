use rusqlite::Connection;
use std::sync::Mutex;

/// Single managed piece of Tauri state: a mutex-guarded SQLite connection.
/// Haleacas is a single-window desktop app with modest concurrency needs,
/// so one connection behind a mutex is deliberately simple rather than
/// pooled.
pub struct AppState {
    pub db: Mutex<Connection>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let db = haleacas_core::db::open()?;
        Ok(Self { db: Mutex::new(db) })
    }
}
