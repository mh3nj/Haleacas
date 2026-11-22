use crate::types::{Dependency, DependencyKind, DependencyManifest};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Parses a `Gemfile` (Ruby/Bundler).
///
/// A Gemfile is executable Ruby, not a declarative manifest — there is
/// no reliable way to fully "parse" one without actually running Ruby.
/// This deliberately does what real static tooling in this space does
/// (e.g. Dependabot's Gemfile parser): a line-oriented scan for `gem
/// "name", "version"` calls, plus tracking `group :x do ... end` blocks
/// well enough to tell development/test gems from production ones. It
/// does not evaluate conditionals, string interpolation, one-line
/// block groups (`group(:test) { gem "x" }`), or gems declared inside
/// `if`/`unless`. Those are undercounted rather than guessed at.
pub fn parse(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("Gemfile")).ok()?;

    let mut dependencies = Vec::new();
    let mut group_stack: Vec<Vec<String>> = Vec::new();

    for raw_line in raw.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(groups) = parse_group_open(line) {
            group_stack.push(groups);
            continue;
        }
        if line == "end" {
            group_stack.pop();
            continue;
        }

        if let Some((name, version)) = parse_gem_line(line) {
            let in_dev_group = group_stack
                .iter()
                .flatten()
                .any(|g| g == "development" || g == "test");
            dependencies.push(Dependency {
                name,
                version_declared: version,
                kind: if in_dev_group { DependencyKind::Dev } else { DependencyKind::Direct },
            });
        }
    }

    Some(DependencyManifest {
        ecosystem: "RubyGems".to_string(),
        manifest_file: "Gemfile".to_string(),
        lockfile_present: root.join("Gemfile.lock").is_file(),
        lockfile_name: root.join("Gemfile.lock").is_file().then(|| "Gemfile.lock".to_string()),
        dependencies,
    })
}

fn strip_comment(line: &str) -> &str {
    // Cheap and deliberately not quote-aware — a `#` inside a gem's
    // version string is exotic enough in practice not to special-case.
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn group_open_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^group\s+((?::\w+\s*,?\s*)+)do\b").unwrap())
}

fn group_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r":(\w+)").unwrap())
}

/// Matches `group :development, :test do` (block form only — the
/// one-line `group(:test) { ... }` form is not recognized).
fn parse_group_open(line: &str) -> Option<Vec<String>> {
    let caps = group_open_re().captures(line)?;
    let symbols_part = caps.get(1)?.as_str();
    let symbols: Vec<String> =
        group_symbol_re().captures_iter(symbols_part).map(|c| c[1].to_string()).collect();
    if symbols.is_empty() {
        None
    } else {
        Some(symbols)
    }
}

fn gem_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^gem\s+["']([^"']+)["'](?:\s*,\s*["']([^"']+)["'])?"#).unwrap()
    })
}

/// Matches `gem "name"`, `gem "name", "~> 1.0"`, and tolerates trailing
/// keyword args (`require: false`, `git: "..."`) by simply not
/// capturing past the first two string literals.
fn parse_gem_line(line: &str) -> Option<(String, String)> {
    let caps = gem_line_re().captures(line)?;
    let name = caps.get(1)?.as_str().to_string();
    let version = caps
        .get(2)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "(unspecified)".to_string());
    Some((name, version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_top_level_gems_with_and_without_version() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("Gemfile"),
            "source \"https://rubygems.org\"\n\n\
             gem \"rails\", \"~> 7.0\"\n\
             gem \"puma\"\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies.len(), 2);
        let rails = manifest.dependencies.iter().find(|d| d.name == "rails").unwrap();
        assert_eq!(rails.version_declared, "~> 7.0");
        assert_eq!(rails.kind, DependencyKind::Direct);
        let puma = manifest.dependencies.iter().find(|d| d.name == "puma").unwrap();
        assert_eq!(puma.version_declared, "(unspecified)");
    }

    #[test]
    fn gems_inside_development_or_test_group_are_dev() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("Gemfile"),
            "gem \"rails\"\n\n\
             group :development, :test do\n\
             \x20\x20gem \"rspec-rails\"\n\
             end\n\n\
             group :production do\n\
             \x20\x20gem \"sidekiq\", \"6.5.0\"\n\
             end\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        let rails = manifest.dependencies.iter().find(|d| d.name == "rails").unwrap();
        assert_eq!(rails.kind, DependencyKind::Direct);
        let rspec = manifest.dependencies.iter().find(|d| d.name == "rspec-rails").unwrap();
        assert_eq!(rspec.kind, DependencyKind::Dev);
        let sidekiq = manifest.dependencies.iter().find(|d| d.name == "sidekiq").unwrap();
        assert_eq!(sidekiq.kind, DependencyKind::Direct);
        assert_eq!(sidekiq.version_declared, "6.5.0");
    }

    #[test]
    fn group_stack_pops_correctly_after_nested_end() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("Gemfile"),
            "group :test do\n\
             \x20\x20gem \"rspec\"\n\
             end\n\
             gem \"after_group\"\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();
        let after = manifest.dependencies.iter().find(|d| d.name == "after_group").unwrap();
        assert_eq!(after.kind, DependencyKind::Direct);
    }

    #[test]
    fn ignores_comments() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("Gemfile"),
            "# gem \"commented_out\"\ngem \"real_gem\" # inline note\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].name, "real_gem");
    }

    #[test]
    fn detects_gemfile_lock() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("Gemfile"), "gem \"rails\"\n").unwrap();
        fs::write(tmp.path().join("Gemfile.lock"), "").unwrap();

        let manifest = parse(tmp.path()).unwrap();
        assert!(manifest.lockfile_present);
    }

    #[test]
    fn missing_manifest_returns_none() {
        let tmp = tempdir().unwrap();
        assert!(parse(tmp.path()).is_none());
    }
}
