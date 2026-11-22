pub mod commands;
pub mod state;
pub mod system;

use state::AppState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let state = AppState::new().map_err(|e| tauri::Error::Anyhow(e))?;
            app.manage(state);

            // Reconcile the OS launch-at-login registration with the
            // saved setting on every start, so the checkbox always
            // reflects reality.
            system::autostart::apply_on_startup(app.handle());

            system::tray::build(app.handle())?;

            // When the OS started us at login (or the user asked for
            // minimized starts), hide straight to the tray instead of
            // popping a window on screen. The tray icon stays alive, and
            // a click on it brings the window back.
            if system::autostart::should_start_minimized(app.handle()) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close behavior follows Settings.close_behavior: MinimizeToTray
            // hides the window (tray Quit is the way out from there),
            // Quit lets the close go through.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let should_quit = window
                        .state::<AppState>()
                        .db
                        .lock()
                        .ok()
                        .and_then(|conn| {
                            haleacas_core::db::settings::load(&conn).ok()
                        })
                        .map(|s| {
                            matches!(
                                s.close_behavior,
                                haleacas_core::types::CloseBehavior::Quit
                            )
                        })
                        .unwrap_or(false);
                    if !should_quit {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::add_repository_root,
            commands::settings::remove_repository_root,
            commands::settings::update_settings,
            commands::repository::get_repositories,
            commands::repository::get_repository,
            commands::repository::get_doctor_findings,
            commands::repository::pick_repository_root_folder,
            commands::repository::scan_repositories,
            commands::repository::open_in_file_manager,
            commands::repository::open_url,
            commands::fixes::get_available_fixes,
            commands::fixes::get_supported_licenses,
            commands::fixes::preview_fix,
            commands::fixes::apply_fix,
            commands::fixes::add_to_gitignore,
            commands::readme::get_readme_sections,
            commands::readme::get_readme_catalog,
            commands::readme::render_readme_markdown,
            commands::readme::preview_readme_save,
            commands::readme::save_readme,
            commands::readme::analyze_readme,
            commands::readme::analyze_readme_text,
            commands::explorer::list_repository_directory,
            commands::explorer::get_explorer_highlights,
            commands::dependencies::get_dependency_report,
            commands::duplicates::get_duplicate_report,
            commands::search::search_repositories,
            commands::editor::open_in_editor,
            commands::timeline::get_repository_timeline,
            commands::timeline::get_stats_at_commit,
            commands::github::get_github_data,
            commands::github::get_repository_releases,
            commands::github::get_github_inbox,
            commands::github::get_github_actions,
            commands::github::connect_github_token,
            commands::github::disconnect_github_token,
            commands::pages::get_pages_report,
            commands::pages::create_pages_index,
            commands::security::list_ignored_findings,
            commands::security::ignore_finding,
            commands::security::unignore_finding,
            commands::ai::test_ai_connection,
            commands::ai::ai_ask,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
