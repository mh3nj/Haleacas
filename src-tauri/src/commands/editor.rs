use crate::state::AppState;
use haleacas_core::{db, editor};
use tauri::State;

/// Launch the user's configured editor (spec sections 25/31) on a path,
/// optionally at a specific line.
///
/// This spawns a real OS process, so it's a genuine trust boundary — but
/// the trust is placed in the *user's own local settings*, not in
/// arbitrary frontend input: `path`/`line` come from data Haleacas itself
/// already knows about (a repository path, a search result), and the
/// binary/args are built as a real argument array via
/// `haleacas_core::editor::launch_args`, never a shell string, so nothing
/// in a weird filename can be interpreted as a shell command.
#[tauri::command]
pub fn open_in_editor(
    path: String,
    line: Option<u32>,
    state: State<AppState>,
) -> Result<(), String> {
    let editor_setting = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::settings::load(&conn).map_err(|e| e.to_string())?.editor
    };

    let Some(editor_setting) = editor_setting else {
        return Err("No editor picked yet. Choose one in Settings.".to_string());
    };

    let (binary, args) = editor::launch_args(&editor_setting, &path, line);

    std::process::Command::new(&binary)
        .args(&args)
        .spawn()
        .map_err(|e| format!("Could not launch \"{binary}\": {e}"))?;

    Ok(())
}
