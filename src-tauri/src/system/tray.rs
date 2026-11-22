//! System tray icon and menu.
//!
//! Haleacas is meant to live in the tray: closing the main window hides it
//! instead of quitting (see the window close handler in lib.rs), and every
//! tray item either shows the window on a specific page or quits the app
//! outright. Page switches are done by emitting a `navigate` event that
//! the frontend router listens for, so the backend never has to know about
//! frontend page components.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const NAV_OVERVIEW: &str = "nav-overview";
const NAV_SCAN: &str = "nav-scan";
const NAV_SEARCH: &str = "nav-search";
const NAV_RECENT: &str = "nav-recent";
const NAV_HEALTH: &str = "nav-health";
const NAV_GITHUB: &str = "nav-github";
const NAV_ACTIONS: &str = "nav-actions";
const NAV_PAGES: &str = "nav-pages";
const NAV_SETTINGS: &str = "nav-settings";
const QUIT: &str = "quit";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, NAV_OVERVIEW, "Open Haleacas", true, None::<&str>)?;
    let scan = MenuItem::with_id(app, NAV_SCAN, "Quick Scan", true, None::<&str>)?;
    let search = MenuItem::with_id(app, NAV_SEARCH, "Search Repositories", true, None::<&str>)?;
    let recent = MenuItem::with_id(app, NAV_RECENT, "Recent Repositories", true, None::<&str>)?;
    let health = MenuItem::with_id(app, NAV_HEALTH, "Repository Health", true, None::<&str>)?;
    let github = MenuItem::with_id(app, NAV_GITHUB, "GitHub Activity", true, None::<&str>)?;
    let actions = MenuItem::with_id(app, NAV_ACTIONS, "GitHub Actions", true, None::<&str>)?;
    let pages = MenuItem::with_id(app, NAV_PAGES, "GitHub Pages", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, NAV_SETTINGS, "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &scan,
            &search,
            &recent,
            &PredefinedMenuItem::separator(app)?,
            &health,
            &github,
            &actions,
            &pages,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        // Left click opens the main window instead of the menu; the full
        // menu is always there on right click. That matches what people
        // expect from a tray app on Windows.
        .show_menu_on_left_click(false)
        .tooltip("Haleacas")
        .on_menu_event(|app, event| {
            let route = match event.id.as_ref() {
                QUIT => {
                    app.exit(0);
                    return;
                }
                NAV_OVERVIEW => "/",
                NAV_SCAN => "/repositories?action=scan",
                NAV_SEARCH => "/search",
                NAV_RECENT => "/repositories",
                NAV_HEALTH => "/health",
                NAV_GITHUB => "/github",
                NAV_ACTIONS => "/actions",
                NAV_PAGES => "/pages",
                NAV_SETTINGS => "/settings",
                _ => "/",
            };
            show_and_navigate(app, route);
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_and_navigate(tray.app_handle(), "/");
            }
        })
        .build(app)?;

    Ok(())
}

fn show_and_navigate(app: &AppHandle, route: &str) {
    if let Some(window) = app.get_webview_window("main") {
        // Un-minimize first so restoring from the taskbar works, then show
        // and focus. Order matters on Windows (show alone leaves a
        // minimized window minimized).
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("navigate", route);
    }
}
