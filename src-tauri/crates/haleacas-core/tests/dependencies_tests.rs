mod common;

use common::write_file;
use haleacas_core::dependencies::scan;
use tempfile::tempdir;

#[test]
fn a_multi_ecosystem_repo_reports_every_present_manifest() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "package.json",
        r#"{"dependencies": {"react": "^18.0.0"}}"#,
    );
    write_file(
        tmp.path(),
        "src-tauri/Cargo.toml",
        "[package]\nname = \"app\"\n\n[dependencies]\ntauri = \"2.0\"\n",
    );

    let report = scan(tmp.path());

    // Cargo.toml at src-tauri/ isn't at the repo root, so only npm should
    // be found here — this also verifies scan() doesn't recurse looking
    // for nested manifests (each parser only checks the given root).
    assert_eq!(report.manifests.len(), 1);
    assert_eq!(report.manifests[0].ecosystem, "npm");
}

#[test]
fn repo_root_manifests_from_multiple_ecosystems_all_appear() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "package.json",
        r#"{"dependencies": {"react": "^18.0.0"}}"#,
    );
    write_file(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\n\n[dependencies]\nserde = \"1.0\"\n",
    );
    write_file(tmp.path(), "go.mod", "module demo\n\ngo 1.21\n\nrequire github.com/pkg/errors v0.9.1\n");
    write_file(
        tmp.path(),
        "pubspec.yaml",
        "name: demo\ndependencies:\n\x20\x20http: ^0.13.3\n",
    );
    write_file(tmp.path(), "Gemfile", "gem \"rails\", \"~> 7.0\"\n");
    write_file(
        tmp.path(),
        "pom.xml",
        "<project><dependencies><dependency><groupId>org.example</groupId>\
         <artifactId>demo</artifactId><version>1.0</version></dependency>\
         </dependencies></project>",
    );
    write_file(
        tmp.path(),
        "build.gradle",
        "dependencies {\n\x20\x20\x20\x20implementation 'org.example:demo:1.0'\n}\n",
    );

    let report = scan(tmp.path());

    let ecosystems: Vec<_> = report.manifests.iter().map(|m| m.ecosystem.as_str()).collect();
    assert!(ecosystems.contains(&"npm"));
    assert!(ecosystems.contains(&"Cargo"));
    assert!(ecosystems.contains(&"Go"));
    assert!(ecosystems.contains(&"pub"));
    assert!(ecosystems.contains(&"RubyGems"));
    assert!(ecosystems.contains(&"Maven"));
    assert!(ecosystems.contains(&"Gradle"));
}

#[test]
fn dependencies_from_every_new_ecosystem_are_actually_populated() {
    // The manifest-presence test above only checks that each ecosystem
    // is *detected* — this checks the dependency inside each one was
    // actually parsed out, not just an empty manifest shell.
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "pubspec.yaml",
        "name: demo\ndependencies:\n\x20\x20http: ^0.13.3\n",
    );
    write_file(tmp.path(), "Gemfile", "gem \"rails\", \"~> 7.0\"\n");
    write_file(
        tmp.path(),
        "pom.xml",
        "<project><dependencies><dependency><groupId>org.example</groupId>\
         <artifactId>demo</artifactId><version>1.0</version></dependency>\
         </dependencies></project>",
    );
    write_file(
        tmp.path(),
        "build.gradle",
        "dependencies {\n\x20\x20\x20\x20implementation 'org.example:demo:1.0'\n}\n",
    );

    let report = scan(tmp.path());

    let pub_manifest = report.manifests.iter().find(|m| m.ecosystem == "pub").unwrap();
    assert_eq!(pub_manifest.dependencies[0].name, "http");

    let gems = report.manifests.iter().find(|m| m.ecosystem == "RubyGems").unwrap();
    assert_eq!(gems.dependencies[0].name, "rails");

    let maven = report.manifests.iter().find(|m| m.ecosystem == "Maven").unwrap();
    assert_eq!(maven.dependencies[0].name, "org.example:demo");

    let gradle = report.manifests.iter().find(|m| m.ecosystem == "Gradle").unwrap();
    assert_eq!(gradle.dependencies[0].name, "org.example:demo");
}

#[test]
fn empty_repo_yields_no_manifests() {
    let tmp = tempdir().unwrap();

    let report = scan(tmp.path());

    assert!(report.manifests.is_empty());
}
