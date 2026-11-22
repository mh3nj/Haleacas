//! Local GitHub Pages readiness check.
//!
//! This is a plain filesystem inspection, no network involved. For every
//! known repository it looks for an entry page a static host could serve
//! (index.html in the root or in the usual publish folders) and reports
//! what it found so the UI can show Ready or Missing per repo.
//!
//! Paths use std::path joins throughout, so the same code works on
//! Windows, macOS and Linux.

use crate::types::{PagesRepoStatus, PagesReport};
use std::path::Path;

/// Candidate entry pages, in the order they are checked. Each entry is a
/// repo-relative path with forward slashes for display.
const ENTRY_CANDIDATES: &[&str] = &[
    "index.html",
    "docs/index.html",
    "public/index.html",
    "dist/index.html",
    "build/index.html",
    "out/index.html",
];

fn join_display(root: &Path, relative: &str) -> std::path::PathBuf {
    let mut p = root.to_path_buf();
    for part in relative.split('/') {
        p.push(part);
    }
    p
}

/// Check one repository checkout on disk.
pub fn check_repo(repo_id: &str, repo_name: &str, path: &str) -> PagesRepoStatus {
    let root = Path::new(path);
    let mut checked = Vec::new();
    let mut found_at = String::new();

    for candidate in ENTRY_CANDIDATES {
        checked.push(candidate.to_string());
        if join_display(root, candidate).is_file() {
            found_at = candidate.to_string();
            break;
        }
    }

    let has_cname = root.join("CNAME").is_file() || root.join("docs").join("CNAME").is_file();
    let has_nojekyll =
        root.join(".nojekyll").is_file() || root.join("docs").join(".nojekyll").is_file();

    let ready = !found_at.is_empty();
    let message = if ready {
        if found_at == "index.html" {
            format!("Ready. Found {found_at} in the repo root, good to publish.")
        } else {
            format!("Ready. Found {found_at}. Point Pages at that folder and you are set.")
        }
    } else if !root.exists() {
        "Heads up. This folder is not on disk anymore, try a fresh scan.".to_string()
    } else {
        "Missing index.html. Add one in the root or in docs/ and Pages will have something to serve.".to_string()
    };

    PagesRepoStatus {
        repo_id: repo_id.to_string(),
        repo_name: repo_name.to_string(),
        path: path.to_string(),
        ready,
        found_at,
        checked,
        has_cname,
        has_nojekyll,
        message,
    }
}

/// Build the whole workspace report from already cached fingerprints.
pub fn report_for(repos: &[(String, String, String)]) -> PagesReport {
    let mut out: Vec<PagesRepoStatus> = repos
        .iter()
        .map(|(id, name, path)| check_repo(id, name, path))
        .collect();
    out.sort_by(|a, b| {
        b.ready
            .cmp(&a.ready)
            .then(a.repo_name.to_lowercase().cmp(&b.repo_name.to_lowercase()))
    });
    let ready = out.iter().filter(|r| r.ready).count();
    let total = out.len();
    PagesReport {
        repos: out,
        total,
        ready,
        missing: total.saturating_sub(ready),
    }
}

/// Minimal starter page used by the "Create index.html" action. Keeps it
/// small and dependency free on purpose.
pub fn starter_html(repo_name: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\" />\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n  <title>{}</title>\n  <style>\n    body {{ font-family: system-ui, sans-serif; max-width: 640px; margin: 4rem auto; padding: 0 1.5rem; }}\n  </style>\n</head>\n<body>\n  <h1>{}</h1>\n  <p>Deployed with GitHub Pages.</p>\n</body>\n</html>\n",
        html_escape(repo_name),
        html_escape(repo_name)
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Write a starter index.html into the repo root. Refuses when one already
/// exists, so this can never silently overwrite real work.
pub fn create_starter_index(path: &str, repo_name: &str) -> Result<String, String> {
    let root = Path::new(path);
    if !root.is_dir() {
        return Err("That folder is not on disk anymore. Run a fresh scan first.".to_string());
    }
    let target = root.join("index.html");
    if target.exists() {
        return Err("index.html already exists. Nothing was overwritten.".to_string());
    }
    let html = starter_html(repo_name);
    std::fs::write(&target, html).map_err(|e| format!("Could not write index.html: {e}"))?;
    Ok(target.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_index_counts_as_ready() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
        let s = check_repo("1", "demo", &dir.path().to_string_lossy());
        assert!(s.ready);
        assert_eq!(s.found_at, "index.html");
    }

    #[test]
    fn docs_index_counts_as_ready() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("docs").join("index.html"), "<h1>hi</h1>").unwrap();
        let s = check_repo("1", "demo", &dir.path().to_string_lossy());
        assert!(s.ready);
        assert_eq!(s.found_at, "docs/index.html");
    }

    #[test]
    fn missing_index_is_reported_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        let s = check_repo("1", "demo", &dir.path().to_string_lossy());
        assert!(!s.ready);
        assert!(s.message.contains("Missing"));
    }

    #[test]
    fn create_starter_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "existing").unwrap();
        let r = create_starter_index(&dir.path().to_string_lossy(), "demo");
        assert!(r.is_err());
    }

    #[test]
    fn create_starter_writes_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let p = &dir.path().to_string_lossy().to_string();
        let written = create_starter_index(p, "demo").unwrap();
        assert!(std::path::Path::new(&written).is_file());
    }
}
