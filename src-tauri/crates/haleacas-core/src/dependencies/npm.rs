use crate::types::{Dependency, DependencyKind, DependencyManifest};
use std::fs;
use std::path::Path;

pub fn parse(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;

    let mut dependencies = Vec::new();
    for (field, kind) in [
        ("dependencies", DependencyKind::Direct),
        ("devDependencies", DependencyKind::Dev),
        ("peerDependencies", DependencyKind::Peer),
    ] {
        if let Some(obj) = value.get(field).and_then(|v| v.as_object()) {
            for (name, version) in obj {
                dependencies.push(Dependency {
                    name: name.clone(),
                    version_declared: version.as_str().unwrap_or("(unspecified)").to_string(),
                    kind,
                });
            }
        }
    }

    let (lockfile_present, lockfile_name) = if root.join("pnpm-lock.yaml").is_file() {
        (true, Some("pnpm-lock.yaml".to_string()))
    } else if root.join("yarn.lock").is_file() {
        (true, Some("yarn.lock".to_string()))
    } else if root.join("package-lock.json").is_file() {
        (true, Some("package-lock.json".to_string()))
    } else {
        (false, None)
    };

    Some(DependencyManifest {
        ecosystem: "npm".to_string(),
        manifest_file: "package.json".to_string(),
        lockfile_present,
        lockfile_name,
        dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_dependencies_devdependencies_and_peerdependencies() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{
                "dependencies": { "react": "^18.0.0" },
                "devDependencies": { "typescript": "^5.0.0" },
                "peerDependencies": { "react-dom": "^18.0.0" }
            }"#,
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies.len(), 3);
        let react = manifest.dependencies.iter().find(|d| d.name == "react").unwrap();
        assert_eq!(react.version_declared, "^18.0.0");
        assert_eq!(react.kind, DependencyKind::Direct);
        let ts = manifest.dependencies.iter().find(|d| d.name == "typescript").unwrap();
        assert_eq!(ts.kind, DependencyKind::Dev);
        let dom = manifest.dependencies.iter().find(|d| d.name == "react-dom").unwrap();
        assert_eq!(dom.kind, DependencyKind::Peer);
    }

    #[test]
    fn detects_pnpm_lockfile_over_others() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(tmp.path().join("package-lock.json"), "").unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert!(manifest.lockfile_present);
        assert_eq!(manifest.lockfile_name.as_deref(), Some("pnpm-lock.yaml"));
    }

    #[test]
    fn no_lockfile_is_reported_honestly() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert!(!manifest.lockfile_present);
        assert_eq!(manifest.lockfile_name, None);
    }

    #[test]
    fn missing_manifest_returns_none() {
        let tmp = tempdir().unwrap();
        assert!(parse(tmp.path()).is_none());
    }
}
