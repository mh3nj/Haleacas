//! Real Git integration via `git2` (spec section 7/9's "Git hygiene").
//!
//! Everything here reads local repository state only — no network access,
//! no pushes/fetches. `remote_url`/`github_slug` are parsed from the
//! repository's configured remote, never fetched from GitHub.

use crate::types::{GitFileChange, GitInfo};
use chrono::{TimeZone, Utc};
use git2::{Repository, Status, StatusOptions};
use std::path::Path;

/// Cap on how many changed-file entries we return in a single fingerprint.
/// A display limit only — `is_dirty` and `changed_files_truncated` remain
/// accurate even when the list itself is cut off.
const CHANGED_FILES_CAP: usize = 200;

pub fn read(path: &Path) -> GitInfo {
    let mut info = GitInfo::default();

    let repo = match Repository::open(path) {
        Ok(r) => r,
        Err(_) => return info,
    };

    // Current branch.
    let head = repo.head().ok();
    if let Some(head) = &head {
        if head.is_branch() {
            info.branch = head.shorthand().map(str::to_string);
        }
    }

    // Primary remote (prefer "origin", fall back to the first configured).
    if let Ok(remotes) = repo.remotes() {
        let name = remotes
            .iter()
            .flatten()
            .find(|n| *n == "origin")
            .or_else(|| remotes.iter().flatten().next())
            .map(str::to_string);

        if let Some(name) = name {
            if let Ok(remote) = repo.find_remote(&name) {
                if let Some(url) = remote.url() {
                    info.remote_url = Some(url.to_string());
                    info.github_slug = github_slug_from_url(url);
                }
            }
        }
    }

    // Working tree status: both the cheap dirty check and the detailed
    // per-file list the Repository Detail page shows.
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        let dirty_entries: Vec<_> = statuses
            .iter()
            .filter(|e| e.status() != Status::CURRENT)
            .collect();
        info.is_dirty = !dirty_entries.is_empty();
        info.changed_files_truncated = dirty_entries.len() > CHANGED_FILES_CAP;
        info.changed_files = dirty_entries
            .into_iter()
            .take(CHANGED_FILES_CAP)
            .filter_map(|entry| {
                entry.path().map(|p| GitFileChange {
                    path: p.to_string(),
                    status: describe_status(entry.status()),
                })
            })
            .collect();
    }

    // Latest commit on HEAD.
    if let Ok(head_commit) = repo.head().and_then(|h| h.peel_to_commit()) {
        info.last_commit_hash = Some(head_commit.id().to_string());
        info.last_commit_message = head_commit
            .summary()
            .map(str::to_string)
            .or_else(|| Some(String::new()));
        info.last_commit_author = head_commit.author().name().map(str::to_string);
        info.last_commit_at = Utc
            .timestamp_opt(head_commit.time().seconds(), 0)
            .single()
            .map(|dt| dt.to_rfc3339());
    }

    // Commit count on HEAD. Capped to keep large-history repos fast; beyond
    // the cap we report the cap and the UI should treat it as a floor, not
    // an exact count.
    const COMMIT_COUNT_CAP: usize = 5_000;
    if let Ok(mut revwalk) = repo.revwalk() {
        if revwalk.push_head().is_ok() {
            info.commit_count = revwalk.take(COMMIT_COUNT_CAP).count();
        }
    }

    // Ahead/behind vs. the current branch's configured upstream, if any.
    // No upstream configured (the common case for a solo local repo, or
    // one that's never been pushed) is not an error — it just means 0/0.
    if let Some(head) = &head {
        if let Some(branch_name) = head.shorthand() {
            if let Ok(local_branch) = repo.find_branch(branch_name, git2::BranchType::Local) {
                if let Ok(upstream) = local_branch.upstream() {
                    if let (Some(local_oid), Some(upstream_oid)) =
                        (head.target(), upstream.get().target())
                    {
                        if let Ok((ahead, behind)) =
                            repo.graph_ahead_behind(local_oid, upstream_oid)
                        {
                            info.ahead = ahead;
                            info.behind = behind;
                        }
                    }
                }
            }
        }
    }

    info
}

/// Translate git2's status bitflags into the single human-readable label
/// the UI shows. A file can technically have several flags set at once
/// (e.g. staged *and* worktree changes); we report the most specific one.
fn describe_status(status: Status) -> String {
    if status.is_conflicted() {
        "conflicted".to_string()
    } else if status.is_wt_new() || status.is_index_new() {
        "new file".to_string()
    } else if status.is_wt_deleted() || status.is_index_deleted() {
        "deleted".to_string()
    } else if status.is_wt_renamed() || status.is_index_renamed() {
        "renamed".to_string()
    } else if status.is_wt_typechange() || status.is_index_typechange() {
        "type changed".to_string()
    } else {
        "modified".to_string()
    }
}

/// Parse an `owner/repo` slug out of a GitHub remote URL, if it is one.
/// Supports both `git@github.com:owner/repo.git` and
/// `https://github.com/owner/repo.git` forms.
fn github_slug_from_url(url: &str) -> Option<String> {
    let url = url.trim_end_matches(".git");

    if let Some(rest) = url.strip_prefix("git@github.com:") {
        return Some(rest.to_string());
    }
    for prefix in ["https://github.com/", "http://github.com/"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            return Some(rest.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ssh_style_github_url() {
        assert_eq!(
            github_slug_from_url("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn parses_https_github_url_with_git_suffix() {
        assert_eq!(
            github_slug_from_url("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn parses_https_github_url_without_git_suffix() {
        assert_eq!(
            github_slug_from_url("https://github.com/owner/repo"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn parses_http_github_url() {
        assert_eq!(
            github_slug_from_url("http://github.com/owner/repo"),
            Some("owner/repo".to_string())
        );
    }

    #[test]
    fn non_github_remote_returns_none() {
        assert_eq!(
            github_slug_from_url("https://gitlab.com/owner/repo.git"),
            None
        );
    }

    #[test]
    fn nested_repo_path_is_preserved() {
        assert_eq!(
            github_slug_from_url("https://github.com/org/mono/repo.git"),
            Some("org/mono/repo".to_string())
        );
    }

    #[test]
    fn describes_new_and_modified_and_deleted_statuses() {
        assert_eq!(describe_status(Status::WT_NEW), "new file");
        assert_eq!(describe_status(Status::INDEX_NEW), "new file");
        assert_eq!(describe_status(Status::WT_MODIFIED), "modified");
        assert_eq!(describe_status(Status::WT_DELETED), "deleted");
        assert_eq!(describe_status(Status::WT_RENAMED), "renamed");
    }
}
