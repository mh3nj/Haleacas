use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::search;
use haleacas_core::types::{SearchOptions, SearchReport};
use tauri::State;

#[tauri::command]
pub fn search_repositories(
    query: String,
    options: SearchOptions,
    state: State<AppState>,
) -> Result<SearchReport, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let repos = db::repositories::list(&conn).map_err(|e| e.to_string())?;
    Ok(search::search(&repos, &query, &options))
}
