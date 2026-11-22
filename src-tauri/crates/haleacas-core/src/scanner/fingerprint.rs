use crate::health::scoring;
use crate::scanner::{detect, git_info, metrics};
use crate::security;
use crate::types::{GitFileChange, GitInfo, HealthReport, RepositoryFingerprint};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn build(path: &Path) -> RepositoryFingerprint {
    let git = git_info::read(path);
    build_with_git(path, git)
}

fn build_with_git(path: &Path, git: GitInfo) -> RepositoryFingerprint {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let description = read_description(path);
    let techs = detect::detect(path);
    let m = metrics::collect(path);
    let security_scan = security::scan(path);

    let mut fp = RepositoryFingerprint {
        id: stable_id(path),
        name,
        path: path.to_string_lossy().to_string(),
        description,
        techs,
        file_count: m.file_count,
        approx_loc: m.approx_loc,
        git,
        license: m.license,
        has_readme: m.has_readme,
        has_gitignore: m.has_gitignore,
        has_ci: m.has_ci,
        has_tests: m.has_tests,
        size_bytes: m.size_bytes,
        scanned_at: Utc::now().to_rfc3339(),
        health: HealthReport::default(),
        security_findings: security_scan.findings,
        security_findings_truncated: security_scan.truncated,
    };

    // Health scoring reads from the fingerprint fields above, so it has to
    // run after they're populated.
    fp.health = scoring::score(&fp);

    fp
}

/// Result of an incremental build: the fingerprint, plus whether it came
/// from the fast "nothing changed" path or a full re-analysis.
/// `was_incremental` is informational only (surfaced in scan progress) —
/// callers should treat `fingerprint` the same either way.
pub struct IncrementalResult {
    pub fingerprint: RepositoryFingerprint,
    pub was_incremental: bool,
}

/// Like `build`, but skips the expensive part (walking and reading every
/// file for LOC/metrics and the security scan) when nothing has
/// detectably changed since `previous` was computed.
///
/// "Detectably changed" means: the HEAD commit moved, or the set of
/// working-tree changes (which files are new/modified/deleted, from Git's
/// own status) differs from last time. Reading Git's status is far
/// cheaper than re-walking and re-reading every file in the repository,
/// and in exchange for that speed it inherits one real gap, documented
/// rather than hidden: editing an already-dirty file's *contents* without
/// changing which status category it's in (e.g. it was "modified" before
/// the edit and is still "modified" after) won't be noticed until the
/// next commit or a file is added/removed. See STATUS.md.
pub fn build_incremental(path: &Path, previous: Option<&RepositoryFingerprint>) -> IncrementalResult {
    let git = git_info::read(path);

    if let Some(prev) = previous {
        if unchanged_since(prev, &git) {
            let mut fp = prev.clone();
            fp.git = git;
            fp.scanned_at = Utc::now().to_rfc3339();
            return IncrementalResult {
                fingerprint: fp,
                was_incremental: true,
            };
        }
    }

    IncrementalResult {
        fingerprint: build_with_git(path, git),
        was_incremental: false,
    }
}

fn unchanged_since(prev: &RepositoryFingerprint, current_git: &GitInfo) -> bool {
    prev.git.last_commit_hash == current_git.last_commit_hash
        && changed_files_signature(&prev.git.changed_files)
            == changed_files_signature(&current_git.changed_files)
}

fn changed_files_signature(files: &[GitFileChange]) -> Vec<(String, String)> {
    let mut sig: Vec<(String, String)> =
        files.iter().map(|f| (f.path.clone(), f.status.clone())).collect();
    sig.sort();
    sig
}

/// A stable id derived from the repository's canonical path, so re-scanning
/// the same repository updates the same database row instead of duplicating
/// it.
fn stable_id(path: &Path) -> String {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canon.to_string_lossy().as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

/// Pull a one-line description from whichever manifest is present, in
/// priority order. Never invented — `None` if nothing declares one.
fn read_description(root: &Path) -> Option<String> {
    if let Ok(raw) = std::fs::read_to_string(root.join("package.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(d) = v.get("description").and_then(|d| d.as_str()) {
                if !d.trim().is_empty() {
                    return Some(d.trim().to_string());
                }
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string(root.join("Cargo.toml")) {
        if let Ok(v) = raw.parse::<toml::Value>() {
            if let Some(d) = v
                .get("package")
                .and_then(|p| p.get("description"))
                .and_then(|d| d.as_str())
            {
                if !d.trim().is_empty() {
                    return Some(d.trim().to_string());
                }
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string(root.join("pyproject.toml")) {
        if let Ok(v) = raw.parse::<toml::Value>() {
            let d = v
                .get("project")
                .and_then(|p| p.get("description"))
                .or_else(|| {
                    v.get("tool")
                        .and_then(|t| t.get("poetry"))
                        .and_then(|p| p.get("description"))
                })
                .and_then(|d| d.as_str());
            if let Some(d) = d {
                if !d.trim().is_empty() {
                    return Some(d.trim().to_string());
                }
            }
        }
    }
    None
}
