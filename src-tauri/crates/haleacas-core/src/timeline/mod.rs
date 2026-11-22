//! Repository Timeline (spec section 22).
//!
//! Two independent local, offline operations:
//!
//! 1. `commits` walks the commit graph and attaches whatever tags point
//!    at each commit — real history, real tag data, nothing from a
//!    GitHub API (releases, which the spec also mentions, need that API
//!    and aren't included here; see STATUS.md).
//! 2. `stats_at_commit` computes file/LOC/structure facts from a
//!    historical commit's **tree object** directly via `git2` — it never
//!    checks out that commit or otherwise touches the user's working
//!    directory. This is what makes a real "before vs. after" comparison
//!    possible without disrupting whatever the user currently has open.

use crate::skip_dirs::is_skipped_dir_name;
use crate::types::{TimelineEntry, TimelineReport, TreeStats};
use chrono::{TimeZone, Utc};
use git2::{ObjectType, Oid, Repository, TreeWalkMode, TreeWalkResult};
use std::collections::HashMap;
use std::path::Path;

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "rb", "java", "kt", "swift", "dart", "c", "h",
    "cpp", "hpp", "cc", "cs", "php", "vue", "svelte", "css", "scss", "html",
];

pub fn commits(root: &Path, limit: usize) -> Result<TimelineReport, String> {
    let repo = Repository::open(root).map_err(|e| e.to_string())?;

    let tags_by_commit = tags_by_commit(&repo);

    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push_head().map_err(|e| e.to_string())?;
    // Topological (ancestry-based), not time-based: two commits made
    // within the same wall-clock second — trivially common in fixtures,
    // and not impossible for a real fast-scripted commit sequence either
    // — would otherwise have an unstable relative order.
    revwalk
        .set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)
        .map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    let mut truncated = false;

    for oid_result in revwalk {
        if entries.len() >= limit {
            truncated = true;
            break;
        }
        let Ok(oid) = oid_result else { continue };
        let Ok(commit) = repo.find_commit(oid) else { continue };

        let hash = oid.to_string();
        let date = Utc
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();

        entries.push(TimelineEntry {
            short_hash: hash[..8.min(hash.len())].to_string(),
            hash,
            message: commit.summary().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            date,
            tags: tags_by_commit.get(&oid).cloned().unwrap_or_default(),
        });
    }

    Ok(TimelineReport { entries, truncated })
}

/// Maps each tagged commit's OID to the tag name(s) pointing at it,
/// resolving both lightweight tags (ref -> commit directly) and
/// annotated tags (ref -> tag object -> commit) to the same result.
fn tags_by_commit(repo: &Repository) -> HashMap<Oid, Vec<String>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();

    let Ok(tag_names) = repo.tag_names(None) else {
        return map;
    };

    for name in tag_names.iter().flatten() {
        let refname = format!("refs/tags/{name}");
        let Ok(reference) = repo.find_reference(&refname) else {
            continue;
        };
        if let Ok(peeled) = reference.peel(ObjectType::Commit) {
            map.entry(peeled.id()).or_default().push(name.to_string());
        }
    }

    map
}

pub fn stats_at_commit(root: &Path, commit_hash: &str) -> Result<TreeStats, String> {
    let repo = Repository::open(root).map_err(|e| e.to_string())?;
    let oid = Oid::from_str(commit_hash).map_err(|e| format!("Invalid commit hash: {e}"))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|_| "Commit not found in this repository.".to_string())?;
    let tree = commit.tree().map_err(|e| e.to_string())?;

    let mut stats = TreeStats {
        file_count: 0,
        approx_loc: 0,
        has_readme: false,
        has_gitignore: false,
        has_ci: false,
        has_tests: false,
    };

    tree.walk(TreeWalkMode::PreOrder, |dir_path, entry| {
        let name = entry.name().unwrap_or("");

        match entry.kind() {
            Some(ObjectType::Tree) => {
                if is_skipped_dir_name(name) {
                    return TreeWalkResult::Skip;
                }
                if dir_path.is_empty() {
                    let lower = name.to_lowercase();
                    if matches!(lower.as_str(), "tests" | "test" | "__tests__" | "spec") {
                        stats.has_tests = true;
                    }
                    if name == ".github" {
                        // Treat any .github/ presence as CI-ish only once
                        // we confirm a workflows/ subdirectory below —
                        // don't flag CI just for issue templates etc.
                    }
                    if name == ".circleci" {
                        stats.has_ci = true;
                    }
                }
                if dir_path == ".github/" && name == "workflows" {
                    stats.has_ci = true;
                }
                TreeWalkResult::Ok
            }
            Some(ObjectType::Blob) => {
                stats.file_count += 1;

                if dir_path.is_empty() {
                    let lower = name.to_lowercase();
                    if lower.starts_with("readme") {
                        stats.has_readme = true;
                    }
                    if name == ".gitignore" {
                        stats.has_gitignore = true;
                    }
                    if name == ".gitlab-ci.yml" {
                        stats.has_ci = true;
                    }
                }

                if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
                    if SOURCE_EXTENSIONS.contains(&ext) {
                        if let Ok(object) = entry.to_object(&repo) {
                            if let Some(blob) = object.as_blob() {
                                if let Ok(text) = std::str::from_utf8(blob.content()) {
                                    stats.approx_loc +=
                                        text.lines().filter(|l| !l.trim().is_empty()).count();
                                }
                            }
                        }
                    }
                }
                TreeWalkResult::Ok
            }
            _ => TreeWalkResult::Ok,
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(stats)
}
