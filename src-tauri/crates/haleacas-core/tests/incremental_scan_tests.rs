mod common;

use common::{init_git_repo, write_file};
use haleacas_core::scanner::fingerprint::{build, build_incremental};
use tempfile::tempdir;

#[test]
fn unchanged_repo_takes_the_incremental_fast_path() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let first = build(tmp.path());
    let second = build_incremental(tmp.path(), Some(&first));

    assert!(second.was_incremental);
    assert_eq!(second.fingerprint.approx_loc, first.approx_loc);
    assert_eq!(second.fingerprint.file_count, first.file_count);
    assert_eq!(second.fingerprint.security_findings.len(), first.security_findings.len());
    assert_eq!(second.fingerprint.health.overall, first.health.overall);
    // Everything about the content is identical; only the id and git
    // snapshot are expected to carry over/refresh, not be recomputed.
    assert_eq!(second.fingerprint.id, first.id);
}

#[test]
fn no_previous_fingerprint_always_does_a_full_build() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);

    let result = build_incremental(tmp.path(), None);

    assert!(!result.was_incremental);
}

#[test]
fn a_new_untracked_file_triggers_a_full_rescan() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);
    let first = build(tmp.path());

    // Add a new source file with real content that should show up in LOC.
    write_file(tmp.path(), "src/new.rs", "fn a() {}\nfn b() {}\nfn c() {}\n");

    let second = build_incremental(tmp.path(), Some(&first));

    assert!(!second.was_incremental);
    assert!(second.fingerprint.approx_loc > first.approx_loc);
    assert!(second.fingerprint.file_count > first.file_count);
}

#[test]
fn a_new_commit_triggers_a_full_rescan_even_with_a_clean_tree() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    let first = build(tmp.path());

    // Commit a new file, leaving the tree clean again afterward.
    write_file(tmp.path(), "src/added.rs", "fn a() {}\n");
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("src/added.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "add file", &tree, &[&parent])
        .unwrap();

    let second = build_incremental(tmp.path(), Some(&first));

    assert!(!second.was_incremental);
    assert!(second.fingerprint.file_count > first.file_count);
}

#[test]
fn a_deleted_tracked_file_triggers_a_full_rescan() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    write_file(tmp.path(), "scratch.txt", "temporary");
    init_git_repo(tmp.path(), None);
    let first = build(tmp.path());

    std::fs::remove_file(tmp.path().join("scratch.txt")).unwrap();

    let second = build_incremental(tmp.path(), Some(&first));

    assert!(!second.was_incremental);
}

#[test]
fn incremental_result_still_reflects_current_git_branch_and_dirty_state() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);
    let first = build(tmp.path());
    assert!(!first.git.is_dirty);

    // Nothing changes, but we should still get a fresh git snapshot back
    // (branch name, timestamps) even on the fast path.
    let second = build_incremental(tmp.path(), Some(&first));
    assert_eq!(second.fingerprint.git.branch, first.git.branch);
}

/// Documents a known, deliberate limitation rather than hiding it: editing
/// an already-dirty file's content without changing which status category
/// it falls into (still "modified", not newly appearing or disappearing)
/// is invisible to the cheap signature check. This is the trade-off
/// written up in STATUS.md — this test exists so a future change to the
/// heuristic doesn't silently alter this behavior without it being
/// noticed.
#[test]
fn known_limitation_editing_an_already_dirty_file_further_is_not_detected() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    write_file(tmp.path(), "src/main.rs", "fn main() {}\n");
    init_git_repo(tmp.path(), None);

    // Make it dirty once, and capture a fingerprint while dirty.
    write_file(tmp.path(), "src/main.rs", "fn main() {\n    println!(\"v1\");\n}\n");
    let first = build(tmp.path());
    assert!(first.git.is_dirty);

    // Edit the same file again — still "modified" in git status, just
    // with different content.
    write_file(
        tmp.path(),
        "src/main.rs",
        "fn main() {\n    println!(\"v2\");\n    println!(\"v3\");\n}\n",
    );

    let second = build_incremental(tmp.path(), Some(&first));

    assert!(
        second.was_incremental,
        "documents the known gap: a second edit to an already-dirty file \
         isn't detected by the status-category signature alone"
    );
}
