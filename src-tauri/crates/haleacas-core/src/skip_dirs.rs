//! Directories every filesystem walk in this crate skips: dependency
//! caches, build output, and VCS/editor metadata. Not a `.gitignore`
//! interpreter — see the known-limitations note in STATUS.md — just a
//! pragmatic, shared denylist so discovery, metrics, the security
//! scanner, and the Repository Explorer all agree on what "noise" means
//! instead of drifting out of sync with their own copies.

pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".nuxt",
    "vendor",
    ".git",
    ".cargo",
    ".gradle",
    ".idea",
    ".vs",
    "bin",
    "obj",
    ".turbo",
];

pub fn is_skipped_dir_name(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}
