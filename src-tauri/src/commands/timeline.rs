use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::timeline;
use haleacas_core::types::{TimelineReport, TreeStats};
use std::path::Path;
use tauri::State;

const COMMIT_LIMIT: usize = 500;

fn repo_path(state: &State<AppState>, id: &str) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::repositories::get_by_id(&conn, id)
        .map_err(|e| e.to_string())?
        .map(|fp| fp.path)
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())
}

#[tauri::command]
pub fn get_repository_timeline(id: String, state: State<AppState>) -> Result<TimelineReport, String> {
    let root = repo_path(&state, &id)?;
    timeline::commits(Path::new(&root), COMMIT_LIMIT)
}

#[tauri::command]
pub fn get_stats_at_commit(
    id: String,
    commit_hash: String,
    state: State<AppState>,
) -> Result<TreeStats, String> {
    let root = repo_path(&state, &id)?;
    timeline::stats_at_commit(Path::new(&root), &commit_hash)
}
