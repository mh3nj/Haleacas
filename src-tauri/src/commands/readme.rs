use crate::state::AppState;
use haleacas_core::db;
use haleacas_core::readme_studio;
use haleacas_core::types::{
    CategoryScore, ReadmeCatalogEntry, ReadmeSavePreview, ReadmeSection, RepositoryFingerprint,
};
use std::fs;
use std::path::Path;
use tauri::State;

fn load_fingerprint(state: &State<AppState>, id: &str) -> Result<RepositoryFingerprint, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::repositories::get_by_id(&conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Repository not found. It may need a fresh scan.".to_string())
}

/// The starter sections a freshly-opened Studio seeds itself with, built
/// from this repository's already-scanned facts (see
/// `readme_studio::generate_sections` for exactly which facts and why
/// only these 4 of the 16 possible section kinds are auto-populated).
#[tauri::command]
pub fn get_readme_sections(id: String, state: State<AppState>) -> Result<Vec<ReadmeSection>, String> {
    let fp = load_fingerprint(&state, &id)?;
    Ok(readme_studio::generate_sections(&fp))
}

/// The full "Add Section" catalog — static, no repository context needed.
#[tauri::command]
pub fn get_readme_catalog() -> Vec<ReadmeCatalogEntry> {
    readme_studio::catalog()
}

/// Pure assembly of the current section list into one Markdown document
/// — no disk access. Called on demand (switching to the Markdown/Preview
/// tab, or before Save) rather than on every keystroke, so editing a
/// section's textarea stays a purely local, instant operation.
#[tauri::command]
pub fn render_readme_markdown(sections: Vec<ReadmeSection>) -> String {
    readme_studio::render_markdown(&sections)
}

/// What Save would write, without writing it — spec section 11's "every
/// modification must be previewable before execution" applies here too,
/// even though this isn't a Doctor fix. `exists` tells the frontend
/// whether to show an overwrite warning.
#[tauri::command]
pub fn preview_readme_save(
    id: String,
    sections: Vec<ReadmeSection>,
    state: State<AppState>,
) -> Result<ReadmeSavePreview, String> {
    let fp = load_fingerprint(&state, &id)?;
    let content = readme_studio::render_markdown(&sections);
    Ok(ReadmeSavePreview {
        target_file: "README.md".to_string(),
        exists: Path::new(&fp.path).join("README.md").is_file(),
        content,
    })
}

/// Writes README.md at the repository root, then re-scans just this one
/// repository so its cached fingerprint (has_readme, health score, ...)
/// reflects the file that was just written — same reasoning as
/// `apply_fix`'s re-scan in `commands/fixes.rs`.
#[tauri::command]
pub fn save_readme(
    id: String,
    sections: Vec<ReadmeSection>,
    state: State<AppState>,
) -> Result<RepositoryFingerprint, String> {
    let fp = load_fingerprint(&state, &id)?;
    let content = readme_studio::render_markdown(&sections);
    fs::write(Path::new(&fp.path).join("README.md"), content).map_err(|e| e.to_string())?;

    let mut refreshed = haleacas_core::scanner::fingerprint::build(Path::new(&fp.path));
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::commands::security::apply_security_ignores(&conn, &mut refreshed)?;
    db::repositories::upsert(&conn, &refreshed).map_err(|e| e.to_string())?;

    Ok(refreshed)
}

/// Spec section 17: analyze the repository's *existing* README.md, if
/// there is one. A missing README isn't an error here — it scores
/// exactly like an empty one, with every check's `detail` explaining why
/// it failed, rather than the frontend getting an error where a report
/// belongs.
#[tauri::command]
pub fn analyze_readme(id: String, state: State<AppState>) -> Result<CategoryScore, String> {
    let fp = load_fingerprint(&state, &id)?;
    let markdown = fs::read_to_string(Path::new(&fp.path).join("README.md")).unwrap_or_default();
    Ok(readme_studio::quality::analyze(&markdown, fp.license.is_some()))
}

/// Same analyzer, but over arbitrary text rather than a file on disk —
/// lets the Studio show a live quality score for the in-progress
/// composition before it's ever saved.
#[tauri::command]
pub fn analyze_readme_text(markdown: String, has_license_file: bool) -> CategoryScore {
    readme_studio::quality::analyze(&markdown, has_license_file)
}
