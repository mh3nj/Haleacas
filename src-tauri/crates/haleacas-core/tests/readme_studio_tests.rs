mod common;

use common::{init_git_repo, write_file};
use haleacas_core::readme_studio::{catalog, generate_sections, quality, render_markdown};
use haleacas_core::scanner::fingerprint::build;
use haleacas_core::types::ReadmeSectionKind;
use tempfile::tempdir;

#[test]
fn generates_renders_and_scores_a_realistic_repository_end_to_end() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("slate");
    std::fs::create_dir_all(&repo_path).unwrap();
    init_git_repo(&repo_path, None);
    write_file(
        &repo_path,
        "package.json",
        r#"{"name": "slate", "description": "A Telegram theme builder."}"#,
    );
    write_file(&repo_path, "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    write_file(&repo_path, "LICENSE", "MIT License\n\nCopyright 2026");

    let fp = build(&repo_path);
    assert_eq!(fp.description.as_deref(), Some("A Telegram theme builder."));
    assert_eq!(fp.license.as_deref(), Some("MIT"));

    let sections = generate_sections(&fp);
    assert_eq!(sections.len(), 4);
    let install = sections.iter().find(|s| s.kind == ReadmeSectionKind::Installation).unwrap();
    assert!(install.content.contains("pnpm install"), "{}", install.content);

    let markdown = render_markdown(&sections);
    assert!(markdown.starts_with("# slate\n\nA Telegram theme builder."));
    assert!(markdown.contains("pnpm install"));
    assert!(markdown.contains("MIT"));

    // The generated skeleton is intentionally minimal (no Usage body, no
    // Screenshots/Contributing sections at all yet) — the quality score
    // should reflect that honestly rather than scoring it as complete.
    let report = quality::analyze(&markdown, fp.license.is_some());
    assert!(report.score < 70, "a skeleton with placeholders shouldn't score high, got {}", report.score);
    let usage_check = report.checks.iter().find(|c| c.label == "Usage example").unwrap();
    assert!(!usage_check.passed);
}

#[test]
fn catalog_covers_every_section_kind_the_spec_names() {
    let entries = catalog();
    let kinds: Vec<_> = entries.iter().map(|e| e.kind).collect();
    for expected in [
        ReadmeSectionKind::Hero,
        ReadmeSectionKind::Description,
        ReadmeSectionKind::Features,
        ReadmeSectionKind::Screenshots,
        ReadmeSectionKind::Demo,
        ReadmeSectionKind::Installation,
        ReadmeSectionKind::QuickStart,
        ReadmeSectionKind::Usage,
        ReadmeSectionKind::Configuration,
        ReadmeSectionKind::Architecture,
        ReadmeSectionKind::Api,
        ReadmeSectionKind::Roadmap,
        ReadmeSectionKind::Contributing,
        ReadmeSectionKind::License,
        ReadmeSectionKind::Support,
        ReadmeSectionKind::Acknowledgements,
    ] {
        assert!(kinds.contains(&expected), "{expected:?} missing from catalog");
    }
}

#[test]
fn studio_and_doctor_fix_agree_on_the_same_install_command() {
    // Both `readme_studio::generate_sections` and the one-click
    // `fixes::readme::generate` Doctor fix derive an install command
    // from the same fact — this guards against the two ever silently
    // drifting apart now that they share `install_command`.
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);
    write_file(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n");

    let fp = build(tmp.path());
    let studio_install = generate_sections(&fp)
        .into_iter()
        .find(|s| s.kind == ReadmeSectionKind::Installation)
        .unwrap()
        .content;
    let doctor_readme = haleacas_core::fixes::readme::generate(&fp);

    assert!(studio_install.contains("cargo build"));
    assert!(doctor_readme.contains("cargo build"));
}
