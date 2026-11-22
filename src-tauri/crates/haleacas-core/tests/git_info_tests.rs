mod common;

use haleacas_core::scanner::git_info::read;
use common::{init_git_repo, write_file};
use std::fs;
use tempfile::tempdir;

#[test]
fn reads_branch_and_remote_from_a_real_repo() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), Some("https://github.com/acme/demo.git"));

    let info = read(tmp.path());

    assert!(info.branch.is_some());
    assert_eq!(info.remote_url.as_deref(), Some("https://github.com/acme/demo.git"));
    assert_eq!(info.github_slug.as_deref(), Some("acme/demo"));
    assert_eq!(info.commit_count, 1);
    assert!(info.last_commit_hash.is_some());
    assert_eq!(info.last_commit_message.as_deref(), Some("initial commit"));
    assert!(!info.is_dirty, "freshly committed repo should be clean");
}

#[test]
fn detects_dirty_working_tree_after_an_untracked_file_is_added() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    // Add a new, untracked file after the initial commit.
    write_file(tmp.path(), "scratch.txt", "not committed");

    let info = read(tmp.path());

    assert!(info.is_dirty);
}

#[test]
fn non_repository_path_returns_empty_default_info() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("just-a-folder")).unwrap();

    let info = read(&tmp.path().join("just-a-folder"));

    assert_eq!(info.branch, None);
    assert_eq!(info.remote_url, None);
    assert_eq!(info.commit_count, 0);
}

#[test]
fn repo_without_a_remote_has_no_github_slug() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let info = read(tmp.path());

    assert_eq!(info.remote_url, None);
    assert_eq!(info.github_slug, None);
}

#[test]
fn changed_files_lists_untracked_and_modified_paths_with_labels() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    write_file(tmp.path(), "keep.txt", "original");
    let repo = init_git_repo(tmp.path(), None);

    // Modify a tracked file and add a new, untracked one.
    write_file(tmp.path(), "keep.txt", "changed");
    write_file(tmp.path(), "new-file.txt", "hello");
    drop(repo);

    let info = read(tmp.path());

    assert!(info.is_dirty);
    let by_path: std::collections::HashMap<_, _> = info
        .changed_files
        .iter()
        .map(|c| (c.path.clone(), c.status.clone()))
        .collect();
    assert_eq!(by_path.get("keep.txt").map(String::as_str), Some("modified"));
    assert_eq!(
        by_path.get("new-file.txt").map(String::as_str),
        Some("new file")
    );
    assert!(!info.changed_files_truncated);
}

#[test]
fn clean_repo_has_no_changed_files() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let info = read(tmp.path());

    assert!(info.changed_files.is_empty());
    assert!(!info.changed_files_truncated);
}

#[test]
fn no_upstream_configured_reports_zero_ahead_behind() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let info = read(tmp.path());

    assert_eq!(info.ahead, 0);
    assert_eq!(info.behind, 0);
}

#[test]
fn diverged_local_upstream_reports_correct_ahead_and_behind() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);

    // Create an "upstream" branch pointing at the current HEAD commit,
    // then advance the current branch by one local commit and the
    // upstream branch by two different commits, so they diverge.
    {
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("upstream-branch", &head_commit, false).unwrap();
    }

    let current_branch_name = repo.head().unwrap().shorthand().unwrap().to_string();

    // Advance the current branch by 1 commit.
    commit_file(&repo, "local-only.txt", "local change");

    // Advance upstream-branch by 2 commits.
    repo.set_head("refs/heads/upstream-branch").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    commit_file(&repo, "upstream-one.txt", "upstream change 1");
    commit_file(&repo, "upstream-two.txt", "upstream change 2");

    // Switch back to the original branch and wire it to track
    // upstream-branch as a local "upstream".
    repo.set_head(&format!("refs/heads/{current_branch_name}"))
        .unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    let mut local_branch = repo
        .find_branch(&current_branch_name, git2::BranchType::Local)
        .unwrap();
    local_branch.set_upstream(Some("upstream-branch")).unwrap();

    let info = read(tmp.path());

    assert_eq!(info.ahead, 1, "current branch has 1 commit upstream lacks");
    assert_eq!(info.behind, 2, "upstream has 2 commits current branch lacks");
}

/// Add and commit a single new file on whatever branch `repo`'s HEAD
/// currently points at.
fn commit_file(repo: &git2::Repository, name: &str, contents: &str) {
    let workdir = repo.workdir().unwrap().to_path_buf();
    std::fs::write(workdir.join(name), contents).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(name)).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, name, &tree, &[&parent])
        .unwrap();
}
