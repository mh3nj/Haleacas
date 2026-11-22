use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::fixes;
use haleacas_core::types::{
    FixKind, FixOption, FixParams, FixPreview, RepositoryFingerprint, SupportedLicense,
};
use tauri::State;

fn load_fingerprint(state: &State<AppState>, id: &str) -> Result<RepositoryFingerprint, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::repositories::get_by_id(&conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())
}

#[tauri::command]
pub fn get_available_fixes(id: String, state: State<AppState>) -> Result<Vec<FixOption>, String> {
    let fp = load_fingerprint(&state, &id)?;
    Ok(fixes::available_fixes(&fp))
}

#[tauri::command]
pub fn get_supported_licenses() -> Vec<SupportedLicense> {
    fixes::licenses::supported_licenses()
}

#[tauri::command]
pub fn preview_fix(
    id: String,
    kind: FixKind,
    params: FixParams,
    state: State<AppState>,
) -> Result<FixPreview, String> {
    let fp = load_fingerprint(&state, &id)?;
    fixes::preview(&fp, kind, &params)
}

/// Apply a fix, then re-scan just this one repository so its cached
/// fingerprint (health score, Doctor findings, has_readme/etc.) reflects
/// the file that was just created — the caller doesn't need to trigger a
/// full re-scan of every configured root for one fix.
#[tauri::command]
pub fn apply_fix(
    id: String,
    kind: FixKind,
    params: FixParams,
    state: State<AppState>,
) -> Result<RepositoryFingerprint, String> {
    let fp = load_fingerprint(&state, &id)?;
    fixes::apply(&fp, kind, &params)?;

    let mut refreshed = haleacas_core::scanner::fingerprint::build(std::path::Path::new(&fp.path));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::commands::security::apply_security_ignores(&conn, &mut refreshed)?;
    db::repositories::upsert(&conn, &refreshed).map_err(|e| e.to_string())?;

    Ok(refreshed)
}

/// Spec section 12's "Add pattern to .gitignore" — appends one pattern to
/// the repository's `.gitignore` (creating it when missing), then
/// re-scans just that repository like `apply_fix` does so `hasGitignore`
/// and health update immediately.
#[tauri::command]
pub fn add_to_gitignore(
    id: String,
    pattern: String,
    state: State<AppState>,
) -> Result<RepositoryFingerprint, String> {
    let fp = load_fingerprint(&state, &id)?;
    fixes::gitignore::append_pattern(std::path::Path::new(&fp.path), &pattern)?;

    let mut refreshed = haleacas_core::scanner::fingerprint::build(std::path::Path::new(&fp.path));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::commands::security::apply_security_ignores(&conn, &mut refreshed)?;
    db::repositories::upsert(&conn, &refreshed).map_err(|e| e.to_string())?;

    Ok(refreshed)
}
