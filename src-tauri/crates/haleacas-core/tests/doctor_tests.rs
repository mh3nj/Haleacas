mod common;

use common::{init_git_repo, write_file};
use haleacas_core::health::doctor::diagnose;
use haleacas_core::scanner::fingerprint::build;
use haleacas_core::types::FindingSeverity;
use tempfile::tempdir;

#[test]
fn a_well_kept_repo_has_no_warning_or_critical_findings() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("tidy");
    std::fs::create_dir_all(&repo_path).unwrap();
    write_file(&repo_path, "README.md", "# Tidy");
    write_file(&repo_path, "LICENSE", "MIT License\n\nCopyright 2026");
    write_file(&repo_path, ".gitignore", "target/\n");
    write_file(&repo_path, "tests/basic.rs", "#[test] fn t() {}");
    write_file(&repo_path, ".github/workflows/ci.yml", "name: CI\n");
    init_git_repo(&repo_path, None);

    let fp = build(&repo_path);
    let findings = diagnose(&fp);

    let bad = findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                FindingSeverity::Critical | FindingSeverity::Warning
            )
        })
        .collect::<Vec<_>>();
    assert!(bad.is_empty(), "unexpected findings: {bad:?}");
}

#[test]
fn missing_readme_and_license_produce_warnings_with_affected_files() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "main.go", "package main");
    write_file(tmp.path(), "go.mod", "module demo\n\ngo 1.21\n");
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());
    let findings = diagnose(&fp);

    let readme_finding = findings
        .iter()
        .find(|f| f.title.contains("README"))
        .expect("expected a README finding");
    assert_eq!(readme_finding.severity, FindingSeverity::Warning);
    assert_eq!(readme_finding.affected_files, vec!["README.md".to_string()]);

    let license_finding = findings
        .iter()
        .find(|f| f.title.contains("license"))
        .expect("expected a license finding");
    assert_eq!(license_finding.severity, FindingSeverity::Warning);
}

#[test]
fn dirty_tree_finding_lists_the_actual_changed_files() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);
    write_file(tmp.path(), "scratch.txt", "wip");

    let fp = build(tmp.path());
    let findings = diagnose(&fp);

    let dirty_finding = findings
        .iter()
        .find(|f| f.title.contains("Uncommitted"))
        .expect("expected an uncommitted-changes finding");
    assert!(dirty_finding.affected_files.contains(&"scratch.txt".to_string()));
}

#[test]
fn every_finding_has_a_non_empty_recommended_action() {
    let tmp = tempdir().unwrap();
    // A minimal, mostly-empty repo, to trigger as many finding types as
    // possible at once.
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());
    let findings = diagnose(&fp);

    assert!(!findings.is_empty(), "expected at least one finding");
    for finding in &findings {
        assert!(
            !finding.recommended_action.trim().is_empty(),
            "{} has no recommended action",
            finding.title
        );
        assert!(
            !finding.why_it_matters.trim().is_empty(),
            "{} has no why_it_matters",
            finding.title
        );
    }
}

#[test]
fn empty_history_repo_gets_a_no_commit_history_warning() {
    let tmp = tempdir().unwrap();
    // Initialize a Git repo with no commits at all (skip the helper's
    // auto-commit by initializing manually).
    git2::Repository::init(tmp.path()).unwrap();

    let fp = build(tmp.path());
    let findings = diagnose(&fp);

    assert!(findings.iter().any(|f| f.title.contains("commit history")));
}

#[test]
fn detected_secret_produces_a_critical_finding_without_the_secret_text() {
    let tmp = tempdir().unwrap();
    let fake_key = "AKIAABCDEFGHIJKLMNOP";
    write_file(tmp.path(), "config.py", &format!("AWS_KEY = \"{fake_key}\"\n"));
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());
    let findings = diagnose(&fp);

    let secret_finding = findings
        .iter()
        .find(|f| f.severity == FindingSeverity::Critical)
        .expect("expected a critical finding for the detected secret");
    assert!(secret_finding.affected_files.contains(&"config.py".to_string()));
    assert!(!secret_finding.explanation.contains(fake_key));
    assert!(!secret_finding.title.contains(fake_key));
}
