use crate::types::{Dependency, DependencyKind, DependencyManifest};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Parses `pom.xml` (Maven).
///
/// Real XML deserialization via `quick-xml`, not a regex scan — unlike
/// Gemfile/build.gradle, pom.xml is genuinely declarative XML with a
/// well-defined schema, so it deserves (and gets) a real parser rather
/// than the heuristic line-scanning used for the executable-DSL formats.
///
/// Deliberately scoped to the top-level `<project><dependencies>` list
/// only. `<dependencyManagement>` entries are *managed* version
/// declarations, not dependencies the module actually pulls in on
/// their own — including them would overstate what's really declared.
/// Dependencies declared only inside `<profiles>` (activated
/// conditionally) are also not scanned, for the same reason the health
/// engine avoids anything that isn't unconditionally true of the repo.
/// Maven has no standard lockfile mechanism (unlike npm/Cargo/Go), so
/// `lockfile_present` is always `false` here rather than checking for
/// a plugin-specific file that may or may not be in use.
pub fn parse(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("pom.xml")).ok()?;
    let project: MavenProject = quick_xml::de::from_str(&raw).ok()?;

    let dependencies = project
        .dependencies
        .map(|d| d.dependency)
        .unwrap_or_default()
        .into_iter()
        .map(|dep| {
            let group = dep.group_id.unwrap_or_else(|| "(unknown group)".to_string());
            let artifact = dep.artifact_id.unwrap_or_else(|| "(unknown artifact)".to_string());
            Dependency {
                name: format!("{group}:{artifact}"),
                version_declared: dep
                    .version
                    .unwrap_or_else(|| "(version managed elsewhere)".to_string()),
                kind: classify_scope(dep.scope.as_deref()),
            }
        })
        .collect();

    Some(DependencyManifest {
        ecosystem: "Maven".to_string(),
        manifest_file: "pom.xml".to_string(),
        lockfile_present: false,
        lockfile_name: None,
        dependencies,
    })
}

/// Maven's `<scope>` — missing scope defaults to `compile`. `runtime`
/// is still a real production dependency (just not needed to compile
/// against), so it's treated the same as `compile`/`Direct`.
fn classify_scope(scope: Option<&str>) -> DependencyKind {
    match scope {
        Some("test") => DependencyKind::Dev,
        Some("provided") | Some("system") => DependencyKind::Build,
        _ => DependencyKind::Direct,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MavenProject {
    #[serde(default)]
    dependencies: Option<MavenDependencies>,
}

#[derive(Debug, Deserialize, Default)]
struct MavenDependencies {
    #[serde(default, rename = "dependency")]
    dependency: Vec<MavenDependency>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MavenDependency {
    group_id: Option<String>,
    artifact_id: Option<String>,
    version: Option<String>,
    scope: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const SAMPLE_POM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>demo</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>org.springframework</groupId>
      <artifactId>spring-core</artifactId>
      <version>5.3.0</version>
    </dependency>
    <dependency>
      <groupId>junit</groupId>
      <artifactId>junit</artifactId>
      <version>4.13.2</version>
      <scope>test</scope>
    </dependency>
    <dependency>
      <groupId>javax.servlet</groupId>
      <artifactId>servlet-api</artifactId>
      <version>2.5</version>
      <scope>provided</scope>
    </dependency>
    <dependency>
      <groupId>com.example</groupId>
      <artifactId>managed-elsewhere</artifactId>
    </dependency>
  </dependencies>
</project>
"#;

    #[test]
    fn parses_dependencies_with_scopes() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("pom.xml"), SAMPLE_POM).unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.ecosystem, "Maven");
        assert_eq!(manifest.dependencies.len(), 4);
        assert!(!manifest.lockfile_present);

        let spring = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "org.springframework:spring-core")
            .unwrap();
        assert_eq!(spring.version_declared, "5.3.0");
        assert_eq!(spring.kind, DependencyKind::Direct);

        let junit = manifest.dependencies.iter().find(|d| d.name == "junit:junit").unwrap();
        assert_eq!(junit.kind, DependencyKind::Dev);

        let servlet =
            manifest.dependencies.iter().find(|d| d.name == "javax.servlet:servlet-api").unwrap();
        assert_eq!(servlet.kind, DependencyKind::Build);

        let managed = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "com.example:managed-elsewhere")
            .unwrap();
        assert_eq!(managed.version_declared, "(version managed elsewhere)");
    }

    #[test]
    fn pom_with_no_dependencies_element_yields_empty_manifest() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("pom.xml"),
            "<project><groupId>x</groupId><artifactId>y</artifactId></project>",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn missing_manifest_returns_none() {
        let tmp = tempdir().unwrap();
        assert!(parse(tmp.path()).is_none());
    }

    #[test]
    fn malformed_xml_returns_none_rather_than_panicking() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("pom.xml"), "<project><unclosed>").unwrap();
        assert!(parse(tmp.path()).is_none());
    }
}
