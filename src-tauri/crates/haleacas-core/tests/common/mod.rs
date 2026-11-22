//! Shared helpers for building throwaway fixture repositories in a temp
//! directory. Each test gets its own `tempfile::TempDir` so fixtures never
//! collide and are cleaned up automatically.
//!
//! Not every test file in this suite uses every helper here (e.g.
//! `detect_tests.rs` only needs `write_file`), so an unused-function
//! warning per test binary is expected and silenced below.
#![allow(dead_code)]

use git2::Repository;
use std::fs;
use std::path::Path;

/// Write a file (and any missing parent directories) under `root`.
pub fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

/// Initialize a real Git repository at `root` with a single commit
/// containing whatever files already exist there, so `git2`-based reads
/// (branch, remote, commit history) have something real to observe.
pub fn init_git_repo(root: &Path, remote_url: Option<&str>) -> Repository {
    let repo = Repository::init(root).expect("git init should succeed");

    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Fixture Author").unwrap();
        config.set_str("user.email", "fixture@example.com").unwrap();
    }

    if let Some(url) = remote_url {
        repo.remote("origin", url).unwrap();
    }

    // Stage everything currently on disk and make an initial commit.
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();
    }

    repo
}
