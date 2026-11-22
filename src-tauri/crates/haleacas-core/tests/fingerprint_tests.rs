mod common;

use haleacas_core::scanner::fingerprint::build;
use common::{init_git_repo, write_file};
use tempfile::tempdir;

#[test]
fn assembles_a_full_fingerprint_for_a_react_repo() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("slate");
    std::fs::create_dir_all(&repo_path).unwrap();

    write_file(
        &repo_path,
        "package.json",
        r#"{
            "name": "slate",
            "description": "A theme builder",
            "dependencies": { "react": "^18.0.0" },
            "devDependencies": { "typescript": "^5.0.0" }
        }"#,
    );
    write_file(&repo_path, "README.md", "# Slate");
    write_file(&repo_path, "src/index.tsx", "export const App = () => null;\n");
    init_git_repo(&repo_path, Some("https://github.com/acme/slate.git"));

    let fp = build(&repo_path);

    assert_eq!(fp.name, "slate");
    assert_eq!(fp.description.as_deref(), Some("A theme builder"));
    assert!(fp.has_readme);
    assert!(fp.license.is_none());
    assert_eq!(fp.git.github_slug.as_deref(), Some("acme/slate"));
    assert!(fp.techs.iter().any(|t| t.name == "React"));
    assert!(fp.techs.iter().any(|t| t.name == "TypeScript"));
    assert!(fp.approx_loc > 0);
    // Stable id should be deterministic for the same canonical path.
    let fp_again = build(&repo_path);
    assert_eq!(fp.id, fp_again.id);
}

#[test]
fn missing_optional_metadata_is_none_rather_than_invented() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().join("bare-repo");
    std::fs::create_dir_all(&repo_path).unwrap();
    write_file(&repo_path, "main.go", "package main\n\nfunc main() {}\n");
    write_file(&repo_path, "go.mod", "module bare\n\ngo 1.21\n");
    init_git_repo(&repo_path, None);

    let fp = build(&repo_path);

    assert_eq!(fp.description, None, "no manifest declared a description");
    assert_eq!(fp.license, None, "no LICENSE file exists");
    assert!(!fp.has_readme);
    assert_eq!(fp.git.remote_url, None);
    assert_eq!(fp.git.github_slug, None);
}

#[test]
fn two_different_repos_get_different_stable_ids() {
    let tmp = tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    init_git_repo(&a, None);
    init_git_repo(&b, None);

    let fp_a = build(&a);
    let fp_b = build(&b);

    assert_ne!(fp_a.id, fp_b.id);
}
