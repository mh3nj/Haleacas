mod common;

use common::{init_git_repo, write_file};
use haleacas_core::health::scoring::score;
use haleacas_core::scanner::fingerprint::build;
use tempfile::tempdir;

#[test]
fn a_well_kept_repo_scores_highly_across_categories() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("tidy");
    std::fs::create_dir_all(&repo_path).unwrap();

    write_file(&repo_path, "README.md", "# Tidy");
    write_file(&repo_path, "LICENSE", "MIT License\n\nCopyright 2026");
    write_file(&repo_path, ".gitignore", "target/\n");
    write_file(&repo_path, "Cargo.toml", "[package]\nname = \"tidy\"");
    write_file(&repo_path, "tests/basic.rs", "#[test] fn t() {}");
    write_file(&repo_path, ".github/workflows/ci.yml", "name: CI\n");
    init_git_repo(&repo_path, None);

    let fp = build(&repo_path);

    assert!(
        fp.health.overall >= 90,
        "expected a high score, got {} — categories: {:?}",
        fp.health.overall,
        fp.health.categories
    );
    for category in &fp.health.categories {
        assert!(
            category.checks.iter().all(|c| c.passed),
            "{} had a failing check: {:?}",
            category.category,
            category.checks
        );
    }
}

#[test]
fn a_bare_repo_scores_low_and_every_check_explains_why() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("bare");
    std::fs::create_dir_all(&repo_path).unwrap();
    write_file(&repo_path, "scratch.txt", "nothing structured here");
    init_git_repo(&repo_path, None);

    let fp = build(&repo_path);

    assert!(
        fp.health.overall < 50,
        "expected a low score, got {}",
        fp.health.overall
    );
    for category in &fp.health.categories {
        for check in &category.checks {
            assert!(
                !check.detail.trim().is_empty(),
                "check {:?} in {} has no explanation",
                check.label,
                category.category
            );
        }
    }
}

#[test]
fn overall_score_is_the_average_of_implemented_categories() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());
    let report = score(&fp);

    let expected_avg = (report.categories.iter().map(|c| c.score as u32).sum::<u32>()
        / report.categories.len() as u32) as u8;
    assert_eq!(report.overall, expected_avg);
}

#[test]
fn lists_categories_not_yet_scored_rather_than_faking_them() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());

    assert!(fp.health.not_yet_scored.contains(&"Dependencies".to_string()));
    assert!(fp
        .health
        .not_yet_scored
        .contains(&"Maintainability".to_string()));
    // Security is scored now (the heuristic secret scanner), so it must
    // NOT appear in not_yet_scored.
    assert!(!fp.health.not_yet_scored.contains(&"Security".to_string()));
    // None of the implemented categories should also appear as unscored.
    for category in &fp.health.categories {
        assert!(!fp.health.not_yet_scored.contains(&category.category));
    }
}

#[test]
fn dirty_working_tree_lowers_git_hygiene_score() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Demo");
    init_git_repo(tmp.path(), None);
    let clean_fp = build(tmp.path());

    write_file(tmp.path(), "uncommitted.txt", "oops");
    let dirty_fp = build(tmp.path());

    let hygiene_score = |fp: &haleacas_core::types::RepositoryFingerprint| {
        fp.health
            .categories
            .iter()
            .find(|c| c.category == "Git hygiene")
            .unwrap()
            .score
    };

    assert!(hygiene_score(&dirty_fp) < hygiene_score(&clean_fp));
}
