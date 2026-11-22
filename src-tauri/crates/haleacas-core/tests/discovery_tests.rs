mod common;

use haleacas_core::scanner::discovery;
use common::{init_git_repo, write_file};
use tempfile::tempdir;

#[test]
fn finds_a_single_top_level_repo() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("my-repo");
    std::fs::create_dir_all(&repo_path).unwrap();
    write_file(&repo_path, "README.md", "# My Repo");
    init_git_repo(&repo_path, None);

    let found = discovery::find_git_repositories(tmp.path());

    assert_eq!(found.len(), 1);
    assert_eq!(found[0], repo_path);
}

#[test]
fn finds_multiple_sibling_repos() {
    let tmp = tempdir().unwrap();
    for name in ["alpha", "beta", "gamma"] {
        let repo_path = tmp.path().join(name);
        std::fs::create_dir_all(&repo_path).unwrap();
        init_git_repo(&repo_path, None);
    }

    let mut found = discovery::find_git_repositories(tmp.path());
    found.sort();

    assert_eq!(found.len(), 3);
}

#[test]
fn does_not_descend_into_node_modules_for_nested_repos() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("app");
    std::fs::create_dir_all(&repo_path).unwrap();
    init_git_repo(&repo_path, None);

    // A dependency vendored under node_modules that happens to carry its
    // own .git directory should NOT be reported as a discovered repo.
    let nested = repo_path.join("node_modules/some-dep");
    std::fs::create_dir_all(&nested).unwrap();
    init_git_repo(&nested, None);

    let found = discovery::find_git_repositories(tmp.path());

    assert_eq!(found, vec![repo_path]);
}

#[test]
fn ignores_plain_directories_with_no_git() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "notes/todo.txt", "just a folder, not a repo");

    let found = discovery::find_git_repositories(tmp.path());

    assert!(found.is_empty());
}

#[test]
fn find_all_deduplicates_overlapping_roots() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("shared");
    std::fs::create_dir_all(&repo_path).unwrap();
    init_git_repo(&repo_path, None);

    // Two configured roots that both contain the same repository.
    let roots = vec![tmp.path().to_path_buf(), tmp.path().to_path_buf()];
    let found = discovery::find_all(&roots);

    assert_eq!(found.len(), 1);
}

#[test]
fn find_all_skips_roots_that_do_not_exist() {
    let tmp = tempdir().unwrap();
    let missing = tmp.path().join("does-not-exist");

    let found = discovery::find_all(&[missing]);

    assert!(found.is_empty());
}
