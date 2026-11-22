use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::explorer;
use haleacas_core::types::{DirectoryListing, ExplorerHighlights};
use std::path::Path;
use tauri::State;

fn repo_path(state: &State<AppState>, id: &str) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::repositories::get_by_id(&conn, id)
        .map_err(|e| e.to_string())?
        .map(|fp| fp.path)
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())
}

#[tauri::command]
pub fn list_repository_directory(
    id: String,
    relative_path: String,
    state: State<AppState>,
) -> Result<DirectoryListing, String> {
    let root = repo_path(&state, &id)?;
    explorer::list_directory(Path::new(&root), &relative_path)
}

#[tauri::command]
pub fn get_explorer_highlights(
    id: String,
    state: State<AppState>,
) -> Result<ExplorerHighlights, String> {
    let root = repo_path(&state, &id)?;
    Ok(explorer::highlights(Path::new(&root)))
}
