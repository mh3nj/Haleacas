use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::health::scoring;
use haleacas_core::security;
use haleacas_core::types::{IgnoredFinding, RepositoryFingerprint};
use rusqlite::Connection;
use tauri::State;

/// Filters a freshly-built fingerprint's security findings against the
/// repository's stored ignore list, then recomputes health scoring so
/// the Security category and overall score reflect the filtered set.
///
/// Called from every place a fingerprint gets freshly built (a full
/// scan, an incremental scan, after applying a fix) — not just the
/// ignore/unignore commands below — so an ignored finding stays hidden
/// everywhere, not just right after you ignore it.
pub fn apply_security_ignores(conn: &Connection, fp: &mut RepositoryFingerprint) -> Result<(), String> {
    let ignored = db::ignored_findings::list(conn, &fp.path).map_err(|e| e.to_string())?;
    if ignored.is_empty() {
        return Ok(());
    }
    fp.security_findings = security::apply_ignore_list(fp.security_findings.clone(), &ignored);
    fp.health = scoring::score(fp);
    Ok(())
}

#[tauri::command]
pub fn list_ignored_findings(id: String, state: State<AppState>) -> Result<Vec<IgnoredFinding>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let fp = db::repositories::get_by_id(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;
    db::ignored_findings::list(&conn, &fp.path).map_err(|e| e.to_string())
}

/// Mark a finding as ignored, then re-run just the (cheap) security scan
/// and re-filter — not a full repository rescan — so the cached
/// fingerprint updates immediately.
#[tauri::command]
pub fn ignore_finding(
    id: String,
    finding: IgnoredFinding,
    state: State<AppState>,
) -> Result<RepositoryFingerprint, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut fp = db::repositories::get_by_id(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;

    db::ignored_findings::add(&conn, &fp.path, &finding).map_err(|e| e.to_string())?;

    let scan = security::scan(std::path::Path::new(&fp.path));
    let ignored = db::ignored_findings::list(&conn, &fp.path).map_err(|e| e.to_string())?;
    fp.security_findings = security::apply_ignore_list(scan.findings, &ignored);
    fp.security_findings_truncated = scan.truncated;
    fp.health = scoring::score(&fp);

    db::repositories::upsert(&conn, &fp).map_err(|e| e.to_string())?;
    Ok(fp)
}

/// Reverse of `ignore_finding` — the finding will show up again on the
/// next scan (immediately, in this case, since this re-scans too).
#[tauri::command]
pub fn unignore_finding(
    id: String,
    finding: IgnoredFinding,
    state: State<AppState>,
) -> Result<RepositoryFingerprint, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut fp = db::repositories::get_by_id(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())?;

    db::ignored_findings::remove(&conn, &fp.path, &finding).map_err(|e| e.to_string())?;

    let scan = security::scan(std::path::Path::new(&fp.path));
    let ignored = db::ignored_findings::list(&conn, &fp.path).map_err(|e| e.to_string())?;
    fp.security_findings = security::apply_ignore_list(scan.findings, &ignored);
    fp.security_findings_truncated = scan.truncated;
    fp.health = scoring::score(&fp);

    db::repositories::upsert(&conn, &fp).map_err(|e| e.to_string())?;
    Ok(fp)
}
