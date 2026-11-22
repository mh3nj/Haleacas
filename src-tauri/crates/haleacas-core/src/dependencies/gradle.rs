use crate::types::{Dependency, DependencyKind, DependencyManifest};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Parses `build.gradle` (Groovy DSL) and/or `build.gradle.kts`
/// (Kotlin DSL).
///
/// Like Gemfile, a Gradle build script is executable code (Groovy or
/// Kotlin), not a declarative manifest — fully understanding one
/// really does require running Gradle itself. This is the same
/// heuristic line-scan approach real static tooling uses for Gradle:
/// match `<configuration> "group:artifact:version"` (or
/// `<configuration>("group:artifact:version")` in the Kotlin DSL).
/// The map-style notation (`implementation group: 'x', name: 'y',
/// version: 'z'`) and version-catalog references (`libs.some.lib`)
/// are not recognized — under-detecting is preferred over guessing.
///
/// A repository can legitimately have both files during a Groovy →
/// Kotlin migration, so — matching `python::parse`'s two-file
/// approach — each file present gets its own manifest rather than one
/// being silently preferred over the other.
pub fn parse(root: &Path) -> Vec<DependencyManifest> {
    let mut manifests = Vec::new();
    if let Some(m) = parse_one(root, "build.gradle") {
        manifests.push(m);
    }
    if let Some(m) = parse_one(root, "build.gradle.kts") {
        manifests.push(m);
    }
    manifests
}

fn parse_one(root: &Path, filename: &str) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join(filename)).ok()?;

    let dependencies = raw
        .lines()
        .filter_map(|line| parse_dependency_line(strip_comment(line.trim())))
        .collect();

    let lockfile_present = root.join("gradle.lockfile").is_file();

    Some(DependencyManifest {
        ecosystem: "Gradle".to_string(),
        manifest_file: filename.to_string(),
        lockfile_present,
        lockfile_name: lockfile_present.then(|| "gradle.lockfile".to_string()),
        dependencies,
    })
}

fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn dependency_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"^(implementation|api|compileOnly|runtimeOnly|annotationProcessor|kapt|ksp|compile|testImplementation|testCompile|testCompileOnly|testRuntimeOnly|debugImplementation|releaseImplementation)\s*[(]?\s*["']([^"']+)["']"#,
        )
        .unwrap()
    })
}

/// Configurations that mean "this is only used for tests" — everything
/// else defaults to `Direct` except the compile-time-only tooling
/// configurations, which map to `Build`.
const DEV_CONFIGS: &[&str] =
    &["testImplementation", "testCompile", "testCompileOnly", "testRuntimeOnly"];
const BUILD_CONFIGS: &[&str] = &["compileOnly", "annotationProcessor", "kapt", "ksp"];

fn parse_dependency_line(line: &str) -> Option<Dependency> {
    let caps = dependency_line_re().captures(line)?;
    let configuration = caps.get(1)?.as_str();
    let coordinate = caps.get(2)?.as_str();

    // Skip anything that isn't a `group:artifact[:version]` coordinate
    // (e.g. `implementation project(":core")` won't match the quoted
    // string this regex requires in the first place, but a quoted
    // string with no colon — a stray literal — is rejected here too).
    let mut parts = coordinate.splitn(3, ':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    if group.is_empty() || artifact.is_empty() {
        return None;
    }
    let version = parts.next();

    let kind = if DEV_CONFIGS.contains(&configuration) {
        DependencyKind::Dev
    } else if BUILD_CONFIGS.contains(&configuration) {
        DependencyKind::Build
    } else {
        DependencyKind::Direct
    };

    Some(Dependency {
        name: format!("{group}:{artifact}"),
        version_declared: version.map(str::to_string).unwrap_or_else(|| "(unspecified)".to_string()),
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_groovy_dsl_string_notation() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("build.gradle"),
            "dependencies {\n\
             \x20\x20\x20\x20implementation 'org.springframework:spring-core:5.3.0'\n\
             \x20\x20\x20\x20testImplementation \"junit:junit:4.13.2\"\n\
             \x20\x20\x20\x20compileOnly 'org.projectlombok:lombok:1.18.24'\n\
             \x20\x20\x20\x20runtimeOnly 'mysql:mysql-connector-java:8.0.30'\n\
             }\n",
        )
        .unwrap();

        let manifests = parse(tmp.path());
        assert_eq!(manifests.len(), 1);
        let manifest = &manifests[0];
        assert_eq!(manifest.manifest_file, "build.gradle");
        assert_eq!(manifest.dependencies.len(), 4);

        let spring = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "org.springframework:spring-core")
            .unwrap();
        assert_eq!(spring.version_declared, "5.3.0");
        assert_eq!(spring.kind, DependencyKind::Direct);

        let junit = manifest.dependencies.iter().find(|d| d.name == "junit:junit").unwrap();
        assert_eq!(junit.kind, DependencyKind::Dev);

        let lombok = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "org.projectlombok:lombok")
            .unwrap();
        assert_eq!(lombok.kind, DependencyKind::Build);

        let mysql = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "mysql:mysql-connector-java")
            .unwrap();
        assert_eq!(mysql.kind, DependencyKind::Direct);
    }

    #[test]
    fn parses_kotlin_dsl_parenthesized_notation() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("build.gradle.kts"),
            "dependencies {\n\
             \x20\x20\x20\x20implementation(\"org.springframework:spring-core:5.3.0\")\n\
             \x20\x20\x20\x20testImplementation(\"junit:junit:4.13.2\")\n\
             }\n",
        )
        .unwrap();

        let manifests = parse(tmp.path());
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].manifest_file, "build.gradle.kts");
        assert_eq!(manifests[0].dependencies.len(), 2);
    }

    #[test]
    fn both_files_present_yields_two_manifests() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("build.gradle"),
            "dependencies { implementation 'a:b:1.0' }\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("build.gradle.kts"),
            "dependencies { implementation(\"c:d:2.0\") }\n",
        )
        .unwrap();

        let manifests = parse(tmp.path());
        assert_eq!(manifests.len(), 2);
    }

    #[test]
    fn dependency_with_no_version_is_unspecified_not_guessed() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("build.gradle"),
            "dependencies {\n\x20\x20\x20\x20implementation 'group:artifact'\n}\n",
        )
        .unwrap();

        let manifest = &parse(tmp.path())[0];
        assert_eq!(manifest.dependencies[0].version_declared, "(unspecified)");
    }

    #[test]
    fn project_dependency_is_not_matched() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("build.gradle"),
            "dependencies {\n\x20\x20\x20\x20implementation project(':core')\n}\n",
        )
        .unwrap();

        let manifest = &parse(tmp.path())[0];
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn detects_gradle_lockfile() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("build.gradle"), "dependencies {}\n").unwrap();
        fs::write(tmp.path().join("gradle.lockfile"), "").unwrap();

        let manifest = &parse(tmp.path())[0];
        assert!(manifest.lockfile_present);
    }

    #[test]
    fn no_files_present_yields_empty_vec() {
        let tmp = tempdir().unwrap();
        assert!(parse(tmp.path()).is_empty());
    }
}
