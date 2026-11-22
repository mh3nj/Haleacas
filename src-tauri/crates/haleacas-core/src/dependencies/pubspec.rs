use crate::types::{Dependency, DependencyKind, DependencyManifest};
use serde_yaml::Value;
use std::fs;
use std::path::Path;

/// Parses `pubspec.yaml` (Dart/Flutter, package manager `pub`).
///
/// Uses a real YAML parser (`serde_yaml`) rather than a hand-rolled
/// line scanner — pubspec.yaml's dependency values are frequently
/// nested maps (`sdk:`, `git:`, `path:`, `hosted:`), and a shallow
/// text scan would misread those as often as it read them correctly.
///
/// `serde_yaml` is tagged `+deprecated` upstream (its original
/// maintainer archived the repo in 2024) but still parses standard
/// YAML correctly; there is no drop-in successor mature enough yet to
/// prefer over it for a read-only parse like this one.
pub fn parse(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("pubspec.yaml")).ok()?;
    let value: Value = serde_yaml::from_str(&raw).ok()?;

    let mut dependencies = Vec::new();
    for (field, kind) in [
        ("dependencies", DependencyKind::Direct),
        ("dev_dependencies", DependencyKind::Dev),
    ] {
        if let Some(Value::Mapping(map)) = value.get(field) {
            for (name, dep_value) in map {
                let Some(name) = name.as_str() else { continue };
                dependencies.push(Dependency {
                    name: name.to_string(),
                    version_declared: describe(dep_value),
                    kind,
                });
            }
        }
    }

    Some(DependencyManifest {
        ecosystem: "pub".to_string(),
        manifest_file: "pubspec.yaml".to_string(),
        lockfile_present: root.join("pubspec.lock").is_file(),
        lockfile_name: root.join("pubspec.lock").is_file().then(|| "pubspec.lock".to_string()),
        dependencies,
    })
}

/// pubspec dependency values come as a bare version string, or a
/// nested map describing an SDK/git/path/hosted dependency with no
/// plain version. Each shape is described honestly rather than
/// guessed at.
fn describe(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Mapping(m) => {
            if let Some(sdk) = m.get("sdk").and_then(|v| v.as_str()) {
                format!("(sdk: {sdk})")
            } else if m.contains_key("git") {
                "(git dependency)".to_string()
            } else if m.contains_key("path") {
                "(path dependency)".to_string()
            } else if let Some(v) = m.get("version").and_then(|v| v.as_str()) {
                v.to_string()
            } else if m.contains_key("hosted") {
                "(hosted dependency)".to_string()
            } else {
                "(unspecified)".to_string()
            }
        }
        Value::Null => "(unspecified)".to_string(),
        _ => "(unspecified)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_plain_and_sdk_and_git_and_path_dependencies() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("pubspec.yaml"),
            "name: demo_app\n\
             dependencies:\n\
             \x20\x20flutter:\n\
             \x20\x20\x20\x20sdk: flutter\n\
             \x20\x20http: ^0.13.3\n\
             \x20\x20some_git_pkg:\n\
             \x20\x20\x20\x20git:\n\
             \x20\x20\x20\x20\x20\x20url: https://example.com/x.git\n\
             \x20\x20some_path_pkg:\n\
             \x20\x20\x20\x20path: ../local_pkg\n\
             dev_dependencies:\n\
             \x20\x20flutter_lints: ^2.0.0\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.ecosystem, "pub");
        assert_eq!(manifest.dependencies.len(), 5);

        let flutter = manifest.dependencies.iter().find(|d| d.name == "flutter").unwrap();
        assert_eq!(flutter.version_declared, "(sdk: flutter)");
        assert_eq!(flutter.kind, DependencyKind::Direct);

        let http = manifest.dependencies.iter().find(|d| d.name == "http").unwrap();
        assert_eq!(http.version_declared, "^0.13.3");

        let git_pkg = manifest.dependencies.iter().find(|d| d.name == "some_git_pkg").unwrap();
        assert_eq!(git_pkg.version_declared, "(git dependency)");

        let path_pkg = manifest.dependencies.iter().find(|d| d.name == "some_path_pkg").unwrap();
        assert_eq!(path_pkg.version_declared, "(path dependency)");

        let lints = manifest.dependencies.iter().find(|d| d.name == "flutter_lints").unwrap();
        assert_eq!(lints.kind, DependencyKind::Dev);
    }

    #[test]
    fn detects_pubspec_lock() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("pubspec.yaml"), "name: demo\n").unwrap();
        fs::write(tmp.path().join("pubspec.lock"), "").unwrap();

        let manifest = parse(tmp.path()).unwrap();
        assert!(manifest.lockfile_present);
        assert_eq!(manifest.lockfile_name.as_deref(), Some("pubspec.lock"));
    }

    #[test]
    fn no_dependencies_section_yields_empty_but_present_manifest() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("pubspec.yaml"), "name: demo\nversion: 1.0.0\n").unwrap();

        let manifest = parse(tmp.path()).unwrap();
        assert!(manifest.dependencies.is_empty());
        assert!(!manifest.lockfile_present);
    }

    #[test]
    fn missing_manifest_returns_none() {
        let tmp = tempdir().unwrap();
        assert!(parse(tmp.path()).is_none());
    }

    #[test]
    fn malformed_yaml_returns_none_rather_than_panicking() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("pubspec.yaml"), "name: [unclosed\n").unwrap();
        assert!(parse(tmp.path()).is_none());
    }
}
