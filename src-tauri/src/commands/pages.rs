use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::pages;
use haleacas_core::types::{PagesReport, RepositoryFingerprint};
use tauri::State;

/// Local Pages readiness for every cached repo. No network, so this loads
/// instantly and is safe to call on page open.
#[tauri::command]
pub fn get_pages_report(state: State<AppState>) -> Result<PagesReport, String> {
    let triples: Vec<(String, String, String)> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::repositories::list(&conn)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|fp: RepositoryFingerprint| (fp.id, fp.name, fp.path))
            .collect()
    };
    Ok(pages::report_for(&triples))
}

/// Create a starter index.html in the repo root. Refuses when one exists.
#[tauri::command]
pub fn create_pages_index(id: String, state: State<AppState>) -> Result<PagesReport, String> {
    let (repo_path, repo_name) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let fp = db::repositories::get_by_id(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;
        (fp.path, fp.name)
    };
    pages::create_starter_index(&repo_path, &repo_name)?;
    get_pages_report(state)
}
