//! Launch-at-login wiring.
//!
//! The `launchAtLogin` flag is stored in the settings blob and applied
//! here, in one place, so the stored flag can never quietly drift from
//! reality:
//!
//! - `apply_on_startup` runs during app setup and reconciles the OS entry
//!   with whatever the saved settings say (so an entry that points at an
//!   old install path, or a stray entry after the flag was turned off
//!   somewhere else, gets fixed up on its own).
//! - `apply` is called from `update_settings` so flipping the checkbox
//!   takes effect right away.
//!
//! This works the same on every desktop OS. The `auto-launch` crate talks
//! to the native spot on each one: the Run key under
//! HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run on Windows (what the
//! Task Manager Startup list reads), Login Items on macOS, and a
//! .desktop file in ~/.config/autostart on Linux. No admin rights needed
//! anywhere. The registered command carries a marker flag so a login
//! launch can be told apart from a normal double click launch.

use anyhow::{anyhow, Context};
use tauri::Manager;

/// Marker flag added to the registered login command. When the app sees
/// this in its own argv it knows it was started by the OS at login, so it
/// can hide straight to the tray instead of popping a window on screen.
pub const AUTOSTART_ARG: &str = "--started-at-login";

/// Older installs registered with this flag. Still honored on startup so
/// people upgrading do not have to toggle the checkbox again.
pub const AUTOSTART_ARG_LEGACY: &str = "--flagged";

/// True when the current process was started by the OS at login.
pub fn launched_at_login() -> bool {
    std::env::args().any(|a| a == AUTOSTART_ARG || a == AUTOSTART_ARG_LEGACY)
}

fn manager() -> anyhow::Result<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe()
        .context("failed to resolve current executable path")?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Haleacas")
        .set_app_path(&exe.to_string_lossy())
        .set_use_launch_agent(false)
        .set_args(&[AUTOSTART_ARG])
        .build()
        .map_err(|e| anyhow!("failed to build auto-launch manager: {e}"))
}

/// Reconcile the OS registration with `enabled`. Called from the settings
/// command when the checkbox is toggled, so errors surface to the UI.
pub fn apply(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let _ = app; // kept for signature symmetry with future platform hooks
    let mgr = manager().map_err(|e| e.to_string())?;
    let is_enabled = mgr.is_enabled().map_err(|e| e.to_string())?;
    match (enabled, is_enabled) {
        (true, false) => mgr.enable().map_err(|e| e.to_string())?,
        (false, true) => mgr.disable().map_err(|e| e.to_string())?,
        _ => {}
    }
    Ok(())
}

/// Called once during setup: sync the OS registration to the persisted
/// setting. Failures are logged but never block startup.
pub fn apply_on_startup(app: &tauri::AppHandle) {
    let enabled = {
        let state = app.state::<crate::state::AppState>();
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("autostart: settings lock poisoned: {e}");
                return;
            }
        };
        match haleacas_core::db::settings::load(&conn) {
            Ok(s) => s.launch_at_login,
            Err(e) => {
                log::warn!("autostart: could not load settings: {e}");
                return;
            }
        }
    };

    let mgr = match manager() {
        Ok(m) => m,
        Err(e) => {
            log::warn!("autostart: {e:#}");
            return;
        }
    };

    let is_enabled = match mgr.is_enabled() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("autostart: could not query current state: {e}");
            return;
        }
    };

    // When staying enabled, re-register once so upgrades from the old
    // --flagged marker pick up the current flag without the user having
    // to flip the checkbox off and on again.
    let result = match (enabled, is_enabled) {
        (true, false) => mgr.enable().map(|_| "enabled"),
        (true, true) => {
            let _ = mgr.disable();
            mgr.enable().map(|_| "refreshed")
        }
        (false, true) => mgr.disable().map(|_| "disabled"),
        _ => Ok("already in sync"),
    };
    match result {
        Ok(what) => log::info!("autostart: launch-at-login {what} (setting: {enabled})"),
        Err(e) => log::warn!("autostart: failed to sync launch-at-login: {e}"),
    }
}

/// True when the main window should start hidden in the tray: either the
/// OS started us at login, or the user asked for minimized starts in
/// Settings. Called once during setup, after settings are loadable.
pub fn should_start_minimized(app: &tauri::AppHandle) -> bool {
    if launched_at_login() {
        return true;
    }
    let state = app.state::<crate::state::AppState>();
    let Ok(conn) = state.db.lock() else {
        return false;
    };
    haleacas_core::db::settings::load(&conn)
        .map(|s| s.start_minimized)
        .unwrap_or(false)
}
