use crate::types::{Dependency, DependencyKind, DependencyManifest};
use std::fs;
use std::path::Path;

/// Returns one manifest per Python dependency file actually present —
/// some Python projects have both `pyproject.toml` and `requirements.txt`
/// (often out of sync with each other), so both are reported separately
/// rather than picking one arbitrarily.
pub fn parse(root: &Path) -> Vec<DependencyManifest> {
    let mut manifests = Vec::new();
    if let Some(m) = parse_pyproject(root) {
        manifests.push(m);
    }
    if let Some(m) = parse_requirements_txt(root) {
        manifests.push(m);
    }
    manifests
}

fn parse_pyproject(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("pyproject.toml")).ok()?;
    let value: toml::Value = raw.parse().ok()?;

    let mut dependencies = Vec::new();

    // PEP 621: [project] dependencies = ["flask>=2.0", ...]
    if let Some(deps) = value
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for entry in deps {
            if let Some(req) = entry.as_str() {
                let (name, version) = parse_pep508(req);
                dependencies.push(Dependency {
                    name,
                    version_declared: version,
                    kind: DependencyKind::Direct,
                });
            }
        }
    }
    // PEP 621 optional groups: [project.optional-dependencies] dev = [...]
    if let Some(groups) = value
        .get("project")
        .and_then(|p| p.get("optional-dependencies"))
        .and_then(|d| d.as_table())
    {
        for group in groups.values() {
            if let Some(deps) = group.as_array() {
                for entry in deps {
                    if let Some(req) = entry.as_str() {
                        let (name, version) = parse_pep508(req);
                        dependencies.push(Dependency {
                            name,
                            version_declared: version,
                            kind: DependencyKind::Dev,
                        });
                    }
                }
            }
        }
    }

    // Poetry: [tool.poetry.dependencies] name = "version" (or table)
    let poetry = value.get("tool").and_then(|t| t.get("poetry"));
    if let Some(deps) = poetry.and_then(|p| p.get("dependencies")).and_then(|d| d.as_table()) {
        for (name, v) in deps {
            if name == "python" {
                continue; // the Python version constraint itself, not a package
            }
            dependencies.push(Dependency {
                name: name.clone(),
                version_declared: describe_poetry_value(v),
                kind: DependencyKind::Direct,
            });
        }
    }
    // Poetry legacy dev-dependencies table, or newer dependency groups.
    if let Some(deps) = poetry
        .and_then(|p| p.get("dev-dependencies"))
        .and_then(|d| d.as_table())
    {
        for (name, v) in deps {
            dependencies.push(Dependency {
                name: name.clone(),
                version_declared: describe_poetry_value(v),
                kind: DependencyKind::Dev,
            });
        }
    }
    if let Some(groups) = poetry.and_then(|p| p.get("group")).and_then(|g| g.as_table()) {
        for group in groups.values() {
            if let Some(deps) = group.get("dependencies").and_then(|d| d.as_table()) {
                for (name, v) in deps {
                    dependencies.push(Dependency {
                        name: name.clone(),
                        version_declared: describe_poetry_value(v),
                        kind: DependencyKind::Dev,
                    });
                }
            }
        }
    }

    if dependencies.is_empty() {
        return None;
    }

    Some(DependencyManifest {
        ecosystem: "Python (pyproject.toml)".to_string(),
        manifest_file: "pyproject.toml".to_string(),
        lockfile_present: root.join("poetry.lock").is_file(),
        lockfile_name: root.join("poetry.lock").is_file().then(|| "poetry.lock".to_string()),
        dependencies,
    })
}

fn describe_poetry_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                v.to_string()
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

fn parse_requirements_txt(root: &Path) -> Option<DependencyManifest> {
    let raw = fs::read_to_string(root.join("requirements.txt")).ok()?;

    let dependencies = raw
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty() && !line.starts_with('#') && !line.starts_with('-')
        })
        .map(|line| {
            let (name, version) = parse_pep508(line);
            Dependency {
                name,
                version_declared: version,
                kind: DependencyKind::Direct,
            }
        })
        .collect();

    Some(DependencyManifest {
        ecosystem: "pip".to_string(),
        manifest_file: "requirements.txt".to_string(),
        lockfile_present: false,
        lockfile_name: None,
        dependencies,
    })
}

/// Parses a PEP 508-ish requirement string ("flask>=2.0", "requests",
/// "black[jupyter]==22.0; python_version >= \"3.8\"") into a (name,
/// declared version) pair. Only the common operators are recognized; an
/// unrecognized shape is reported as "(unspecified)" rather than guessed.
fn parse_pep508(req: &str) -> (String, String) {
    // Drop an environment marker if present.
    let req = req.split(';').next().unwrap_or(req).trim();
    // Drop extras like `black[jupyter]`.
    let req = match req.find('[') {
        Some(bracket_start) => {
            let after_bracket = req[bracket_start..].find(']').map(|i| bracket_start + i + 1);
            match after_bracket {
                Some(end) => format!("{}{}", &req[..bracket_start], &req[end..]),
                None => req.to_string(),
            }
        }
        None => req.to_string(),
    };
    let req = req.trim();

    const OPERATORS: &[&str] = &["===", "~=", "==", ">=", "<=", "!=", ">", "<"];
    for op in OPERATORS {
        if let Some(idx) = req.find(op) {
            let name = req[..idx].trim().to_string();
            let version = req[idx..].trim().to_string();
            if !name.is_empty() {
                return (name, version);
            }
        }
    }

    if req.is_empty() {
        ("(unknown)".to_string(), "(unspecified)".to_string())
    } else {
        (req.to_string(), "(unspecified)".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_pep621_dependencies_array() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\ndependencies = [\"flask>=2.0\", \"requests\"]\n",
        )
        .unwrap();

        let manifest = parse_pyproject(tmp.path()).unwrap();

        let flask = manifest.dependencies.iter().find(|d| d.name == "flask").unwrap();
        assert_eq!(flask.version_declared, ">=2.0");
        let requests = manifest.dependencies.iter().find(|d| d.name == "requests").unwrap();
        assert_eq!(requests.version_declared, "(unspecified)");
    }

    #[test]
    fn parses_poetry_style_dependencies() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("pyproject.toml"),
            "[tool.poetry.dependencies]\npython = \"^3.10\"\nflask = \"^2.0\"\n",
        )
        .unwrap();

        let manifest = parse_pyproject(tmp.path()).unwrap();

        // The python version constraint itself is not a package dependency.
        assert!(!manifest.dependencies.iter().any(|d| d.name == "python"));
        let flask = manifest.dependencies.iter().find(|d| d.name == "flask").unwrap();
        assert_eq!(flask.version_declared, "^2.0");
    }

    #[test]
    fn parses_requirements_txt_with_comments_and_options() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("requirements.txt"),
            "# core deps\nflask==2.0.0\nrequests>=2.0\n\n-e .\n-r other.txt\nblack[jupyter]==22.0\n",
        )
        .unwrap();

        let manifest = parse_requirements_txt(tmp.path()).unwrap();

        assert_eq!(manifest.dependencies.len(), 3);
        let flask = manifest.dependencies.iter().find(|d| d.name == "flask").unwrap();
        assert_eq!(flask.version_declared, "==2.0.0");
        let black = manifest.dependencies.iter().find(|d| d.name == "black").unwrap();
        assert_eq!(black.version_declared, "==22.0");
    }

    #[test]
    fn strips_environment_markers() {
        let (name, version) = parse_pep508("black>=22.0; python_version >= \"3.8\"");
        assert_eq!(name, "black");
        assert_eq!(version, ">=22.0");
    }

    #[test]
    fn bare_name_with_no_version_is_unspecified_not_guessed() {
        let (name, version) = parse_pep508("requests");
        assert_eq!(name, "requests");
        assert_eq!(version, "(unspecified)");
    }

    #[test]
    fn both_files_present_yields_two_manifests() {
        let tmp = tempdir().unwrap();
        std::fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\ndependencies = [\"flask>=2.0\"]\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "flask==2.0.0\n").unwrap();

        let manifests = parse(tmp.path());

        assert_eq!(manifests.len(), 2);
    }
}
