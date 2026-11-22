//! Recursive Git repository discovery (spec section 7).
//!
//! We walk each configured root looking for `.git` directories. Once a
//! repository boundary is found we do not keep descending *through* it
//! looking for nested repos inside dependency/build folders — nested repos
//! that live under paths like `node_modules/**/.git` are noise, not
//! projects the user wants indexed. Directories that are expensive/noisy to
//! traverse (dependency caches, build output) are skipped outright.

use crate::skip_dirs::is_skipped_dir_name;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_git_repositories(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        if !entry.file_type().is_dir() {
            return true;
        }
        let name = entry.file_name().to_string_lossy();
        !is_skipped_dir_name(&name)
    });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        if entry.path().join(".git").is_dir() {
            found.push(entry.path().to_path_buf());
        }
    }

    found
}

/// Discover repositories under every configured root, deduplicating by
/// canonical path in case roots overlap.
pub fn find_all(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut all = Vec::new();

    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for repo in find_git_repositories(root) {
            let canon = repo.canonicalize().unwrap_or_else(|_| repo.clone());
            if seen.insert(canon.clone()) {
                all.push(repo);
            }
        }
    }

    all
}
