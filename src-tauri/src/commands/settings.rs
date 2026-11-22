use crate::state::AppState;
use crate::system::autostart;
use haleacas_core::db;
use haleacas_core::types::{RepositoryRoot, Settings};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::settings::load(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_repository_root(path: String, state: State<AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut settings = db::settings::load(&conn).map_err(|e| e.to_string())?;

    if !settings.roots.iter().any(|r| r.path == path) {
        settings.roots.push(RepositoryRoot { path });
    }
    db::settings::save(&conn, &settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn remove_repository_root(path: String, state: State<AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut settings = db::settings::load(&conn).map_err(|e| e.to_string())?;

    settings.roots.retain(|r| r.path != path);
    db::settings::save(&conn, &settings).map_err(|e| e.to_string())?;
    db::repositories::remove_by_path_prefix(&conn, &path).map_err(|e| e.to_string())?;
    db::ignored_findings::remove_by_path_prefix(&conn, &path).map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn update_settings(
    settings: Settings,
    state: State<AppState>,
    app: AppHandle,
) -> Result<Settings, String> {
    // Compare against the previously-persisted value so we only touch the
    // OS registration when the flag actually changed.
    let previous_launch_at_login = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let previous = db::settings::load(&conn).map_err(|e| e.to_string())?;
        db::settings::save(&conn, &settings).map_err(|e| e.to_string())?;
        previous.launch_at_login
    };

    if settings.launch_at_login != previous_launch_at_login {
        autostart::apply(&app, settings.launch_at_login)?;
    }

    Ok(settings)
}
