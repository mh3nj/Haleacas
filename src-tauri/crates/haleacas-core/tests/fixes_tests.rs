mod common;

use common::{init_git_repo, write_file};
use haleacas_core::fixes::{apply, available_fixes, preview};
use haleacas_core::scanner::fingerprint::build;
use haleacas_core::types::{FixKind, FixParams};
use tempfile::tempdir;

#[test]
fn missing_files_are_reported_as_available() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());
    let fixes = available_fixes(&fp);

    for fix in &fixes {
        assert!(fix.available, "{:?} should be available on a bare repo", fix.kind);
        assert!(fix.unavailable_reason.is_none());
    }
}

#[test]
fn existing_files_are_reported_as_unavailable_with_a_reason() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "README.md", "# Already here");
    write_file(tmp.path(), ".gitignore", "already here too");
    init_git_repo(tmp.path(), None);

    let fp = build(tmp.path());
    let fixes = available_fixes(&fp);

    let readme_fix = fixes.iter().find(|f| f.kind == FixKind::CreateReadmeSkeleton).unwrap();
    assert!(!readme_fix.available);
    assert!(readme_fix.unavailable_reason.as_ref().unwrap().contains("README.md"));

    let gitignore_fix = fixes.iter().find(|f| f.kind == FixKind::CreateGitignore).unwrap();
    assert!(!gitignore_fix.available);
}

#[test]
fn preview_does_not_touch_disk() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);
    let fp = build(tmp.path());

    let p = preview(&fp, FixKind::CreateGitignore, &FixParams::default()).unwrap();

    assert!(!tmp.path().join(".gitignore").exists());
    assert_eq!(p.target_file, ".gitignore");
    assert!(!p.content.is_empty());
}

#[test]
fn apply_writes_exactly_the_previewed_content() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);
    let fp = build(tmp.path());
    let params = FixParams::default();

    let previewed = preview(&fp, FixKind::CreateGitignore, &params).unwrap();
    let written_path = apply(&fp, FixKind::CreateGitignore, &params).unwrap();
    let on_disk = std::fs::read_to_string(&written_path).unwrap();

    assert_eq!(on_disk, previewed.content);
}

#[test]
fn apply_refuses_to_overwrite_an_existing_file() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), ".gitignore", "hand-written, do not touch");
    init_git_repo(tmp.path(), None);
    let fp = build(tmp.path());

    let result = apply(&fp, FixKind::CreateGitignore, &FixParams::default());

    assert!(result.is_err());
    let on_disk = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert_eq!(on_disk, "hand-written, do not touch");
}

#[test]
fn apply_refuses_even_if_file_appeared_after_the_fingerprint_was_built() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);
    // fp.hasGitignore is false at this point (no .gitignore exists yet).
    let fp = build(tmp.path());

    // Simulate the file being created by something else in the meantime.
    write_file(tmp.path(), ".gitignore", "created after the scan");

    let result = apply(&fp, FixKind::CreateGitignore, &FixParams::default());

    assert!(
        result.is_err(),
        "apply() must re-check the filesystem, not trust the stale fingerprint flag"
    );
    let on_disk = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert_eq!(on_disk, "created after the scan");
}

#[test]
fn create_license_requires_a_license_id_and_holder() {
    let tmp = tempdir().unwrap();
    init_git_repo(tmp.path(), None);
    let fp = build(tmp.path());

    assert!(preview(&fp, FixKind::CreateLicense, &FixParams::default()).is_err());

    let missing_holder = FixParams {
        license_id: Some("mit".to_string()),
        copyright_holder: None,
    };
    assert!(preview(&fp, FixKind::CreateLicense, &missing_holder).is_err());

    let complete = FixParams {
        license_id: Some("mit".to_string()),
        copyright_holder: Some("Jane Doe".to_string()),
    };
    let result = preview(&fp, FixKind::CreateLicense, &complete).unwrap();
    assert!(result.content.contains("Jane Doe"));
    assert_eq!(result.target_file, "LICENSE");
}

#[test]
fn readme_skeleton_reflects_the_actual_repo_name() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("slate");
    std::fs::create_dir_all(&repo_path).unwrap();
    init_git_repo(&repo_path, None);
    let fp = build(&repo_path);

    let p = preview(&fp, FixKind::CreateReadmeSkeleton, &FixParams::default()).unwrap();

    assert!(p.content.starts_with("# slate"));
}
