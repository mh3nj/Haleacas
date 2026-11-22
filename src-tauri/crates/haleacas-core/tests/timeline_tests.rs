mod common;

use common::{init_git_repo, write_file};
use git2::Repository;
use haleacas_core::timeline::{commits, stats_at_commit};
use tempfile::tempdir;

fn commit_file(repo: &Repository, name: &str, contents: &str, message: &str) {
    let workdir = repo.workdir().unwrap().to_path_buf();
    std::fs::create_dir_all(workdir.join(name).parent().unwrap()).unwrap();
    std::fs::write(workdir.join(name), contents).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(name)).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
        .unwrap();
}

#[test]
fn commits_are_returned_oldest_first() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    commit_file(&repo, "a.rs", "fn a() {}", "add a");
    commit_file(&repo, "b.rs", "fn b() {}", "add b");

    let report = commits(tmp.path(), 100).unwrap();

    assert_eq!(report.entries.len(), 3);
    assert_eq!(report.entries[0].message, "initial commit");
    assert_eq!(report.entries[1].message, "add a");
    assert_eq!(report.entries[2].message, "add b");
    assert!(!report.truncated);
}

#[test]
fn short_hash_is_a_prefix_of_the_full_hash() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let report = commits(tmp.path(), 10).unwrap();

    let entry = &report.entries[0];
    assert_eq!(entry.short_hash.len(), 8);
    assert!(entry.hash.starts_with(&entry.short_hash));
}

#[test]
fn limit_truncates_and_sets_the_flag() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    commit_file(&repo, "a.rs", "fn a() {}", "add a");
    commit_file(&repo, "b.rs", "fn b() {}", "add b");

    let report = commits(tmp.path(), 2).unwrap();

    assert_eq!(report.entries.len(), 2);
    assert!(report.truncated);
}

#[test]
fn lightweight_tag_is_attached_to_its_commit() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag_lightweight("v1.0.0", head.as_object(), false).unwrap();

    let report = commits(tmp.path(), 10).unwrap();

    assert_eq!(report.entries[0].tags, vec!["v1.0.0".to_string()]);
}

#[test]
fn annotated_tag_resolves_to_the_underlying_commit() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = repo.signature().unwrap();
    repo.tag("v2.0.0", head.as_object(), &sig, "release", false).unwrap();

    let report = commits(tmp.path(), 10).unwrap();

    assert_eq!(report.entries[0].tags, vec!["v2.0.0".to_string()]);
}

#[test]
fn commit_with_no_tags_has_an_empty_tag_list() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let report = commits(tmp.path(), 10).unwrap();

    assert!(report.entries[0].tags.is_empty());
}

#[test]
fn stats_at_commit_reflects_that_point_in_history_not_the_working_tree() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    let first_hash = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();

    commit_file(&repo, "src/main.rs", "fn main() {}\nfn helper() {}\n", "add main");
    let second_hash = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();

    // Now dirty the actual working tree with something uncommitted, to
    // prove stats_at_commit reads history, not the live filesystem.
    write_file(tmp.path(), "uncommitted.rs", "fn uncommitted() {}\n");

    let before = stats_at_commit(tmp.path(), &first_hash).unwrap();
    let after = stats_at_commit(tmp.path(), &second_hash).unwrap();

    assert_eq!(before.file_count, 1); // just README.md
    assert_eq!(after.file_count, 2); // README.md + src/main.rs
    assert!(after.approx_loc > before.approx_loc);
}

#[test]
fn stats_at_commit_never_touches_the_working_directory() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    commit_file(&repo, "src/main.rs", "fn main() {}\n", "add main");
    let hash = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();

    // Go back and compute stats for the FIRST commit, which never had
    // src/main.rs...
    let first = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .parent(0)
        .unwrap()
        .id()
        .to_string();
    let _ = stats_at_commit(tmp.path(), &first).unwrap();

    // ...and confirm the actual working directory file is untouched:
    // stats_at_commit must not have checked anything out.
    assert!(tmp.path().join("src/main.rs").is_file());
    let _ = hash;
}

#[test]
fn detects_tests_directory_and_ci_at_a_given_commit() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    let repo = init_git_repo(tmp.path(), None);
    commit_file(&repo, "tests/basic.rs", "#[test] fn t() {}", "add tests");
    commit_file(&repo, ".github/workflows/ci.yml", "name: CI", "add ci");
    let hash = repo.head().unwrap().peel_to_commit().unwrap().id().to_string();

    let stats = stats_at_commit(tmp.path(), &hash).unwrap();

    assert!(stats.has_tests);
    assert!(stats.has_ci);
}

#[test]
fn invalid_commit_hash_is_a_clear_error_not_a_panic() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let result = stats_at_commit(tmp.path(), "not-a-real-hash");

    assert!(result.is_err());
}

#[test]
fn well_formed_but_nonexistent_hash_is_also_a_clear_error() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let result = stats_at_commit(tmp.path(), "0000000000000000000000000000000000000000");

    assert!(result.is_err());
}
