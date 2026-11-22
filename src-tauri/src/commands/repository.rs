use haleacas_core::db;
use haleacas_core::scanner::{discovery, fingerprint};
use crate::state::AppState;
use haleacas_core::types::{RepositoryFingerprint, ScanProgress};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
/// Return whatever fingerprints are currently cached, without touching the
/// filesystem. Fast path for opening the app / Repositories page.
#[tauri::command]
pub fn get_repositories(state: State<AppState>) -> Result<Vec<RepositoryFingerprint>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::repositories::list(&conn).map_err(|e| e.to_string())
}

/// Fetch a single cached fingerprint by id, for the Repository Detail page.
/// `None` if it hasn't been scanned (or was scanned, then its root removed).
#[tauri::command]
pub fn get_repository(
    id: String,
    state: State<AppState>,
) -> Result<Option<RepositoryFingerprint>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::repositories::get_by_id(&conn, &id).map_err(|e| e.to_string())
}

/// Repository Doctor findings for a cached repository (spec section 10).
/// Derived purely from already-collected fingerprint data — no extra
/// filesystem or Git access, so this is cheap even for large repos.
#[tauri::command]
pub fn get_doctor_findings(
    id: String,
    state: State<AppState>,
) -> Result<Option<Vec<haleacas_core::types::DoctorFinding>>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let fp = db::repositories::get_by_id(&conn, &id).map_err(|e| e.to_string())?;
    Ok(fp.map(|fp| haleacas_core::health::doctor::diagnose(&fp)))
}

/// Reveal a folder in the OS's file manager (spec section 25's "Open
/// Folder" quick action).
#[tauri::command]
pub fn open_in_file_manager(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}

/// Open a URL in the system's default browser (spec section 25's "Open
/// GitHub" quick action). Never called with anything but a URL Haleacas
/// itself constructed from a parsed GitHub remote — never raw user input.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Open a native folder picker and, if the user chose one, add it as a
/// repository root.
#[tauri::command]
pub async fn pick_repository_root_folder(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.map(|p| p.to_string()))
}

/// Walk every configured root, discover Git repositories, and compute a
/// fresh fingerprint for each — emitting `scan-progress` events as it goes
/// (spec section 26: indexing must stay visible but never dominate the UI).
///
/// Uses incremental scanning (spec section 26: "avoid rescanning
/// unchanged files unnecessarily"): for each repository already in the
/// cache, a cheap Git-only check decides whether anything detectably
/// changed before paying for the expensive full walk-and-read. See
/// `haleacas_core::scanner::fingerprint::build_incremental`.
///
/// This runs the filesystem/Git work on a blocking thread so the async
/// Tauri command doesn't stall the IPC runtime, and only touches the
/// database from the calling task once each fingerprint is ready.
#[tauri::command]
pub async fn scan_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<RepositoryFingerprint>, String> {
    let roots: Vec<PathBuf> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::settings::load(&conn)
            .map_err(|e| e.to_string())?
            .roots
            .into_iter()
            .map(|r| PathBuf::from(r.path))
            .collect()
    };

    let app_for_blocking = app.clone();
    let repo_paths = tauri::async_runtime::spawn_blocking(move || {
        let paths = discovery::find_all(&roots);
        let _ = app_for_blocking.emit(
            "scan-progress",
            ScanProgress {
                current: 0,
                total: paths.len(),
                current_path: None,
                skipped: false,
            },
        );
        paths
    })
    .await
    .map_err(|e| e.to_string())?;

    let total = repo_paths.len();
    let mut results = Vec::with_capacity(total);

    for (i, path) in repo_paths.into_iter().enumerate() {
        let path_string = path.to_string_lossy().to_string();
        let previous = {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            db::repositories::get_by_path(&conn, &path_string).map_err(|e| e.to_string())?
        };

        let path_for_blocking = path.clone();
        let incremental = tauri::async_runtime::spawn_blocking(move || {
            fingerprint::build_incremental(&path_for_blocking, previous.as_ref())
        })
        .await
        .map_err(|e| e.to_string())?;

        let mut fp = incremental.fingerprint;

        {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            // Idempotent whether this was a full rebuild (fresh,
            // unfiltered security_findings) or the incremental fast path
            // (already-filtered findings carried over from the cache) —
            // filtering an already-filtered list a second time is a
            // no-op.
            crate::commands::security::apply_security_ignores(&conn, &mut fp)?;
            db::repositories::upsert(&conn, &fp).map_err(|e| e.to_string())?;
        }

        let _ = app.emit(
            "scan-progress",
            ScanProgress {
                current: i + 1,
                total,
                current_path: Some(fp.name.clone()),
                skipped: incremental.was_incremental,
            },
        );

        results.push(fp);
    }

    Ok(results)
}
