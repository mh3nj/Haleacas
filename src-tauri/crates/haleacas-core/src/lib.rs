//! Haleacas' framework-agnostic domain logic: repository scanning, Git
//! integration, and local persistence.
//!
//! Deliberately has **no** dependency on Tauri or any GUI toolkit — this
//! crate is plain Rust, so it compiles and links quickly on its own (no
//! webkit2gtk/gtk3 in its dependency graph) and can be unit/integration
//! tested in isolation from the desktop shell. The `app` package (in the
//! parent `src-tauri/`) wraps this crate's functions in `#[tauri::command]`s
//! and wires them to IPC/state/tray — see spec section 37: "keep domain
//! logic separated from UI."

pub mod ai;
pub mod db;
pub mod dependencies;
pub mod duplicates;
pub mod editor;
pub mod explorer;
pub mod github;
pub mod fixes;
pub mod health;
pub mod pages;
pub mod readme_studio;
pub mod scanner;
pub mod search;
pub mod security;
pub mod skip_dirs;
pub mod timeline;
pub mod types;
