use crate::types::{Dependency, DependencyKind, DependencyManifest};
use std::fs;
use std::path::Path;

pub fn parse(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let value: toml::Value = raw.parse().ok()?;

    let mut dependencies = Vec::new();
    for (section, kind) in [
        ("dependencies", DependencyKind::Direct),
        ("dev-dependencies", DependencyKind::Dev),
        ("build-dependencies", DependencyKind::Build),
    ] {
        if let Some(table) = value.get(section).and_then(|v| v.as_table()) {
            for (name, dep_value) in table {
                dependencies.push(Dependency {
                    name: name.clone(),
                    version_declared: describe(dep_value),
                    kind,
                });
            }
        }
    }

    Some(DependencyManifest {
        ecosystem: "Cargo".to_string(),
        manifest_file: "Cargo.toml".to_string(),
        lockfile_present: root.join("Cargo.lock").is_file(),
        lockfile_name: root.join("Cargo.lock").is_file().then(|| "Cargo.lock".to_string()),
        dependencies,
    })
}

/// Cargo dependency values come in several shapes — a bare version
/// string, a table with `version`, or a table describing a path/git/
/// workspace dependency with no version at all. Each is described
/// honestly rather than guessing a version that isn't there.
fn describe(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                v.to_string()
            } else if t.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
                "(workspace)".to_string()
            } else if t.contains_key("path") {
                "(path dependency)".to_string()
            } else if t.contains_key("git") {
                "(git dependency)".to_string()
            } else {
                "(unspecified)".to_string()
            }
        }
        _ => "(unspecified)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_plain_string_versions() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies[0].name, "serde");
        assert_eq!(manifest.dependencies[0].version_declared, "1.0");
        assert_eq!(manifest.dependencies[0].kind, DependencyKind::Direct);
    }

    #[test]
    fn parses_table_form_with_version_key() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n\n[dependencies]\nserde = { version = \"1.0\", features = [\"derive\"] }\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies[0].version_declared, "1.0");
    }

    #[test]
    fn path_dependency_is_labeled_not_guessed() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n\n[dependencies]\nlocal-crate = { path = \"../local-crate\" }\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies[0].version_declared, "(path dependency)");
    }

    #[test]
    fn separates_dependencies_dev_dependencies_and_build_dependencies() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n\n\
             [dependencies]\nserde = \"1.0\"\n\n\
             [dev-dependencies]\ntempfile = \"3.10\"\n\n\
             [build-dependencies]\ncc = \"1.0\"\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies.len(), 3);
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.name == "tempfile" && d.kind == DependencyKind::Dev));
        assert!(manifest
            .dependencies
            .iter()
            .any(|d| d.name == "cc" && d.kind == DependencyKind::Build));
    }

    #[test]
    fn detects_cargo_lock() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        std::fs::write(tmp.path().join("Cargo.lock"), "").unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert!(manifest.lockfile_present);
    }
}
