//! Dependency Intelligence (spec section 13).
//!
//! Parses whichever manifests are actually present — never invents a
//! dependency, a version, or an ecosystem the repository doesn't have
//! evidence for. Covers all 8 ecosystems the spec names: npm, Cargo,
//! Python (both PEP 621 and Poetry styles in pyproject.toml, plus
//! requirements.txt), Go, pub (pubspec.yaml), RubyGems (Gemfile),
//! Maven (pom.xml), and Gradle (build.gradle / build.gradle.kts).
//!
//! Manifest formats get parsers matched to their actual nature:
//! package.json/Cargo.toml/pyproject.toml/pubspec.yaml/pom.xml are
//! genuinely declarative (JSON/TOML/YAML/XML) and get real format
//! parsers (`serde_json`, `toml`, `serde_yaml`, `quick-xml`). go.mod,
//! Gemfile, and build.gradle(.kts) are not declarative — a Go module
//! file has its own bespoke grammar, and a Gemfile/build.gradle is
//! literally executable Ruby/Groovy/Kotlin — so those get the
//! heuristic line-oriented scanning real static tooling in this space
//! (e.g. Dependabot) uses for the same formats, documented per-module
//! with exactly what they do and don't recognize.
//!
//! No network access happens here at all — every dependency shown
//! reflects only what's declared locally. There is no outdated/freshness
//! checking (the spec's "Known locally / Last checked online / Unknown"
//! distinction) since that needs the optional online refresh spec
//! section 13 describes, which isn't built yet.

pub mod cargo;
pub mod gemfile;
pub mod go;
pub mod gradle;
pub mod npm;
pub mod pom;
pub mod pubspec;
pub mod python;

use crate::types::DependencyReport;
use std::path::Path;

pub fn scan(root: &Path) -> DependencyReport {
    let mut manifests = Vec::new();

    if let Some(m) = npm::parse(root) {
        manifests.push(m);
    }
    if let Some(m) = cargo::parse(root) {
        manifests.push(m);
    }
    manifests.extend(python::parse(root));
    if let Some(m) = go::parse(root) {
        manifests.push(m);
    }
    if let Some(m) = pubspec::parse(root) {
        manifests.push(m);
    }
    if let Some(m) = gemfile::parse(root) {
        manifests.push(m);
    }
    if let Some(m) = pom::parse(root) {
        manifests.push(m);
    }
    manifests.extend(gradle::parse(root));

    DependencyReport { manifests }
}
