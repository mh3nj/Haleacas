mod common;

use common::write_file;
use haleacas_core::explorer::{highlights, list_directory};
use tempfile::tempdir;

#[test]
fn lists_immediate_children_only_not_recursively() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    write_file(tmp.path(), "src/main.rs", "fn main() {}");
    write_file(tmp.path(), "src/nested/deep.rs", "// deep");

    let listing = list_directory(tmp.path(), "").unwrap();
    let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();

    assert!(names.contains(&"README.md"));
    assert!(names.contains(&"src"));
    // Nothing from inside src/ should appear at the root listing.
    assert!(!names.contains(&"main.rs"));
    assert!(!names.contains(&"nested"));
}

#[test]
fn directories_sort_before_files_then_alphabetically() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "zeta.txt", "z");
    write_file(tmp.path(), "alpha.txt", "a");
    write_file(tmp.path(), "beta/file.txt", "b");

    let listing = list_directory(tmp.path(), "").unwrap();
    let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();

    assert_eq!(names, vec!["beta", "alpha.txt", "zeta.txt"]);
}

#[test]
fn can_navigate_into_a_subdirectory() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "src/main.rs", "fn main() {}");
    write_file(tmp.path(), "src/lib.rs", "// lib");

    let listing = list_directory(tmp.path(), "src").unwrap();
    let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();

    assert_eq!(names.len(), 2);
    assert!(names.contains(&"main.rs"));
    assert!(names.contains(&"lib.rs"));
}

#[test]
fn rejects_parent_directory_traversal() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(tmp.path(), "outside-secret.txt", "should not be reachable");

    let result = list_directory(&repo, "../");

    assert!(result.is_err());
}

#[test]
fn rejects_traversal_via_nested_dotdot_segments() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    write_file(&repo, "src/main.rs", "fn main() {}");
    write_file(tmp.path(), "outside-secret.txt", "should not be reachable");

    let result = list_directory(&repo, "src/../../");

    assert!(result.is_err());
}

#[test]
fn marks_known_config_files() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "package.json", "{}");
    write_file(tmp.path(), "notes.txt", "not config");

    let listing = list_directory(tmp.path(), "").unwrap();
    let pkg = listing.entries.iter().find(|e| e.name == "package.json").unwrap();
    let notes = listing.entries.iter().find(|e| e.name == "notes.txt").unwrap();

    assert!(pkg.is_config);
    assert!(!notes.is_config);
}

#[test]
fn marks_test_files_and_directories() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "tests/basic.rs", "#[test] fn t() {}");
    write_file(tmp.path(), "src/main.rs", "fn main() {}");

    let root_listing = list_directory(tmp.path(), "").unwrap();
    let tests_dir = root_listing.entries.iter().find(|e| e.name == "tests").unwrap();
    assert!(tests_dir.is_test);

    let inner_listing = list_directory(tmp.path(), "tests").unwrap();
    let basic = inner_listing.entries.iter().find(|e| e.name == "basic.rs").unwrap();
    assert!(basic.is_test);
}

#[test]
fn highlights_finds_the_largest_files() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "small.txt", "hi");
    write_file(tmp.path(), "big.txt", &"x".repeat(10_000));

    let h = highlights(tmp.path());

    assert_eq!(h.largest_files[0].name, "big.txt");
}

#[test]
fn highlights_skip_noisy_directories() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "node_modules/dep/index.js", &"x".repeat(50_000));
    write_file(tmp.path(), "src/main.rs", "fn main() {}");

    let h = highlights(tmp.path());

    assert!(h.largest_files.iter().all(|f| !f.relative_path.contains("node_modules")));
    assert!(h.recently_modified.iter().all(|f| !f.relative_path.contains("node_modules")));
}

#[test]
fn highlights_caps_each_list_at_ten() {
    let tmp = tempdir().unwrap();
    for i in 0..15 {
        write_file(tmp.path(), &format!("file{i}.txt"), "content");
    }

    let h = highlights(tmp.path());

    assert_eq!(h.largest_files.len(), 10);
    assert_eq!(h.recently_modified.len(), 10);
}

#[test]
fn relative_paths_use_forward_slashes() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "a/b/c.txt", "content");

    let h = highlights(tmp.path());

    assert_eq!(h.largest_files[0].relative_path, "a/b/c.txt");
}
