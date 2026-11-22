mod common;

use haleacas_core::scanner::metrics::collect;
use common::write_file;
use tempfile::tempdir;

#[test]
fn counts_non_blank_lines_in_recognized_source_files() {
    let tmp = tempdir().unwrap();
    // 3 non-blank lines, 1 blank line -> should count as 3.
    write_file(tmp.path(), "src/index.ts", "const a = 1;\n\nconst b = 2;\nconst c = 3;\n");
    // Not a recognized source extension -> should not contribute to LOC.
    write_file(tmp.path(), "notes.txt", "one\ntwo\nthree\nfour\nfive\n");

    let m = collect(tmp.path());

    assert_eq!(m.approx_loc, 3);
    assert_eq!(m.file_count, 2);
}

#[test]
fn detects_readme_case_insensitively() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "Readme.MD", "# Title");

    let m = collect(tmp.path());

    assert!(m.has_readme);
}

#[test]
fn detects_gitignore_ci_and_tests_presence() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), ".gitignore", "node_modules/\n");
    write_file(tmp.path(), ".github/workflows/ci.yml", "name: CI\n");
    write_file(tmp.path(), "tests/basic.rs", "#[test] fn t() {}");

    let m = collect(tmp.path());

    assert!(m.has_gitignore);
    assert!(m.has_ci);
    assert!(m.has_tests);
}

#[test]
fn missing_optional_files_are_reported_as_absent_not_guessed() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "src/main.rs", "fn main() {}");

    let m = collect(tmp.path());

    assert!(!m.has_readme);
    assert!(!m.has_gitignore);
    assert!(!m.has_ci);
    assert!(!m.has_tests);
    assert_eq!(m.license, None);
}

#[test]
fn identifies_mit_license_text() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "LICENSE",
        "MIT License\n\nCopyright (c) 2026 Someone\n",
    );

    let m = collect(tmp.path());

    assert_eq!(m.license, Some("MIT".to_string()));
}

#[test]
fn skips_noisy_directories_when_counting() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "src/index.ts", "console.log(1);\n");
    // Should be skipped entirely, not counted toward file_count/LOC.
    write_file(
        tmp.path(),
        "node_modules/dep/index.js",
        "a\nb\nc\nd\ne\nf\ng\nh\n",
    );

    let m = collect(tmp.path());

    assert_eq!(m.file_count, 1);
    assert_eq!(m.approx_loc, 1);
}

#[test]
fn size_bytes_reflects_actual_file_sizes() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "a.txt", "12345"); // 5 bytes
    write_file(tmp.path(), "b.txt", "1234567890"); // 10 bytes

    let m = collect(tmp.path());

    assert_eq!(m.size_bytes, 15);
}
