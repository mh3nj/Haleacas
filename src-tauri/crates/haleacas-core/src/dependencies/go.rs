use crate::types::{Dependency, DependencyKind, DependencyManifest};
use std::fs;
use std::path::Path;

pub fn parse(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("go.mod")).ok()?;

    let mut dependencies = Vec::new();
    let mut in_require_block = false;

    for line in raw.lines() {
        let line = line.trim();

        if line.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        if in_require_block && line == ")" {
            in_require_block = false;
            continue;
        }

        let entry = if in_require_block {
            Some(line)
        } else if let Some(rest) = line.strip_prefix("require ") {
            Some(rest.trim())
        } else {
            None
        };

        let Some(entry) = entry else { continue };
        if entry.is_empty() {
            continue;
        }

        if let Some((module, version, is_indirect)) = parse_require_line(entry) {
            dependencies.push(Dependency {
                name: module,
                version_declared: version,
                kind: if is_indirect {
                    DependencyKind::Transitive
                } else {
                    DependencyKind::Direct
                },
            });
        }
    }

    Some(DependencyManifest {
        ecosystem: "Go".to_string(),
        manifest_file: "go.mod".to_string(),
        lockfile_present: root.join("go.sum").is_file(),
        lockfile_name: root.join("go.sum").is_file().then(|| "go.sum".to_string()),
        dependencies,
    })
}

/// Parses one `module version [// indirect]` line from inside or outside
/// a `require (...)` block. Returns `None` for lines that don't look like
/// a module requirement at all (this function is never handed `module`/
/// `go`/`toolchain`/`replace`/`exclude` lines by the caller, but stays
/// defensive regardless).
fn parse_require_line(line: &str) -> Option<(String, String, bool)> {
    let (code, comment) = match line.split_once("//") {
        Some((c, comment)) => (c.trim(), Some(comment.trim())),
        None => (line.trim(), None),
    };

    let mut parts = code.split_whitespace();
    let module = parts.next()?;
    let version = parts.next()?;

    if !version.starts_with('v') {
        return None;
    }

    let is_indirect = comment.map(|c| c.contains("indirect")).unwrap_or(false);
    Some((module.to_string(), version.to_string(), is_indirect))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_a_require_block() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("go.mod"),
            "module example.com/demo\n\ngo 1.21\n\nrequire (\n\
             \tgithub.com/pkg/errors v0.9.1\n\
             \tgolang.org/x/sys v0.5.0 // indirect\n\
             )\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies.len(), 2);
        let direct = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "github.com/pkg/errors")
            .unwrap();
        assert_eq!(direct.version_declared, "v0.9.1");
        assert_eq!(direct.kind, DependencyKind::Direct);

        let indirect = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "golang.org/x/sys")
            .unwrap();
        assert_eq!(indirect.kind, DependencyKind::Transitive);
    }

    #[test]
    fn parses_single_line_require_statements() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("go.mod"),
            "module example.com/demo\n\ngo 1.21\n\nrequire github.com/pkg/errors v0.9.1\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].name, "github.com/pkg/errors");
    }

    #[test]
    fn ignores_module_go_and_toolchain_directives() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("go.mod"),
            "module example.com/demo\n\ngo 1.21\n\ntoolchain go1.21.5\n",
        )
        .unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn detects_go_sum_lockfile() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("go.mod"), "module demo\n\ngo 1.21\n").unwrap();
        std::fs::write(tmp.path().join("go.sum"), "").unwrap();

        let manifest = parse(tmp.path()).unwrap();

        assert!(manifest.lockfile_present);
    }
}
