mod common;

use haleacas_core::scanner::detect::detect;
use common::write_file;
use tempfile::tempdir;

fn names(techs: &[haleacas_core::types::TechDetection]) -> Vec<String> {
    techs.iter().map(|t| t.name.clone()).collect()
}

#[test]
fn detects_react_typescript_via_package_json() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "package.json",
        r#"{
            "name": "demo",
            "dependencies": { "react": "^18.0.0" },
            "devDependencies": { "typescript": "^5.0.0", "vite": "^5.0.0" }
        }"#,
    );
    write_file(tmp.path(), "package-lock.json", "{}");

    let found = names(&detect(tmp.path()));

    assert!(found.contains(&"Node.js".to_string()));
    assert!(found.contains(&"React".to_string()));
    assert!(found.contains(&"TypeScript".to_string()));
    assert!(found.contains(&"Vite".to_string()));
    assert!(found.contains(&"npm".to_string()));
    // Evidence-based: Vue was never mentioned, so it must not appear.
    assert!(!found.contains(&"Vue".to_string()));
}

#[test]
fn detects_tauri_from_src_tauri_directory_even_without_dependency() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "package.json", r#"{"name": "demo"}"#);
    write_file(tmp.path(), "src-tauri/Cargo.toml", "[package]\nname = \"app\"");

    let found = names(&detect(tmp.path()));

    assert!(found.contains(&"Tauri".to_string()));
}

#[test]
fn detects_rust_and_tauri_from_cargo_toml() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "Cargo.toml",
        r#"
        [package]
        name = "demo"

        [dependencies]
        tauri = "2.0"
        tokio = "1.0"
        "#,
    );

    let found = names(&detect(tmp.path()));

    assert!(found.contains(&"Rust".to_string()));
    assert!(found.contains(&"Tauri".to_string()));
    assert!(found.contains(&"Tokio".to_string()));
    assert!(!found.contains(&"Axum".to_string()));
}

#[test]
fn detects_python_and_framework_from_requirements_txt() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "requirements.txt", "flask==2.0.0\nrequests==2.0.0\n");

    let found = names(&detect(tmp.path()));

    assert!(found.contains(&"Python".to_string()));
    assert!(found.contains(&"Flask".to_string()));
    assert!(!found.contains(&"Django".to_string()));
}

#[test]
fn detects_go_from_go_mod() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "go.mod", "module example.com/demo\n\ngo 1.21\n");

    let found = names(&detect(tmp.path()));

    assert_eq!(found, vec!["Go".to_string()]);
}

#[test]
fn detects_flutter_from_pubspec_yaml() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "pubspec.yaml",
        "name: demo\nenvironment:\n  sdk: '>=2.12.0'\ndependencies:\n  flutter:\n    sdk: flutter\n",
    );

    let found = names(&detect(tmp.path()));

    assert!(found.contains(&"Dart".to_string()));
    assert!(found.contains(&"Flutter".to_string()));
}

#[test]
fn empty_directory_yields_no_detections() {
    let tmp = tempdir().unwrap();

    let found = detect(tmp.path());

    assert!(found.is_empty(), "expected no detections, got {found:?}");
}

#[test]
fn every_detection_carries_non_empty_evidence() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "package.json",
        r#"{"dependencies": {"react": "^18.0.0"}}"#,
    );

    for tech in detect(tmp.path()) {
        assert!(
            !tech.evidence.trim().is_empty(),
            "{} was detected with no evidence",
            tech.name
        );
    }
}
