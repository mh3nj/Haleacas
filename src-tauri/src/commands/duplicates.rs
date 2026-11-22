use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::duplicates;
use haleacas_core::types::DuplicateReport;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub fn get_duplicate_report(id: String, state: State<AppState>) -> Result<DuplicateReport, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let fp = db::repositories::get_by_id(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;

    Ok(duplicates::scan(Path::new(&fp.path)))
}
