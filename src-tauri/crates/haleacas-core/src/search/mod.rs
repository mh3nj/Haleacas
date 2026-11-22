//! Global Search (spec section 14).
//!
//! This is on-demand search, not a maintained persistent index — every
//! call walks the repositories being searched and reads matching files
//! fresh. Spec section 14 frames this as "index all configured
//! repositories," which more naturally means a persistent, incrementally
//! updated search index; that's a meaningfully bigger feature (storage,
//! invalidation on file change, etc.) than this round scoped in. What's
//! here is genuinely useful — real local text search with real results —
//! just not backed by a database index yet. See STATUS.md.
//!
//! "Search for symbols where feasible" (also spec section 14) isn't
//! implemented either: symbol-aware search needs per-language parsing,
//! which is out of scope here. This is plain substring text search only.
//!
//! No repository content is stored anywhere by this module — every
//! search reads files live and only returns the matching lines.

use crate::skip_dirs::is_skipped_dir_name;
use crate::types::{RepositoryFingerprint, SearchMatch, SearchOptions, SearchReport};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const MAX_TOTAL_MATCHES: usize = 200;
const MAX_MATCHES_PER_FILE: usize = 20;
const MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_LINE_TEXT_LENGTH: usize = 300;

const SKIP_FILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "Gemfile.lock",
    "poetry.lock",
    "composer.lock",
];

pub fn search(
    repos: &[RepositoryFingerprint],
    query: &str,
    options: &SearchOptions,
) -> SearchReport {
    if query.trim().is_empty() {
        return SearchReport::default();
    }

    let needle = if options.case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    let mut matches = Vec::new();
    let mut truncated = false;
    let mut repositories_searched = 0usize;

    'repos: for repo in repos {
        if let Some(filter_id) = &options.repo_id {
            if &repo.id != filter_id {
                continue;
            }
        }
        let root = Path::new(&repo.path);
        if !root.is_dir() {
            continue;
        }
        repositories_searched += 1;

        let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
            if !entry.file_type().is_dir() {
                return true;
            }
            !is_skipped_dir_name(&entry.file_name().to_string_lossy())
        });

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy();
            if SKIP_FILES.contains(&file_name.as_ref()) {
                continue;
            }
            if let Some(ext_filter) = &options.extension {
                let matches_ext = entry
                    .path()
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case(ext_filter.as_str()))
                    .unwrap_or(false);
                if !matches_ext {
                    continue;
                }
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.len() > MAX_FILE_SIZE_BYTES {
                continue;
            }
            let Ok(contents) = fs::read_to_string(entry.path()) else {
                continue;
            };

            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");

            if let Some(path_filter) = &options.path_substring {
                let filter = path_filter.trim();
                if !filter.is_empty() {
                    let haystack = if options.case_sensitive {
                        relative.clone()
                    } else {
                        relative.to_lowercase()
                    };
                    let want = if options.case_sensitive {
                        filter.to_string()
                    } else {
                        filter.to_lowercase()
                    };
                    if !haystack.contains(&want) {
                        continue;
                    }
                }
            }

            let mut matches_in_file = 0usize;
            for (line_no, line) in contents.lines().enumerate() {
                let haystack = if options.case_sensitive {
                    line.to_string()
                } else {
                    line.to_lowercase()
                };
                if haystack.contains(&needle) {
                    let mut line_text = line.trim().to_string();
                    if line_text.len() > MAX_LINE_TEXT_LENGTH {
                        line_text.truncate(MAX_LINE_TEXT_LENGTH);
                        line_text.push('…');
                    }
                    matches.push(SearchMatch {
                        repo_id: repo.id.clone(),
                        repo_name: repo.name.clone(),
                        repo_path: repo.path.clone(),
                        file: relative.clone(),
                        line: line_no + 1,
                        line_text,
                    });
                    matches_in_file += 1;
                    if matches_in_file >= MAX_MATCHES_PER_FILE {
                        // We don't know whether this file had more
                        // matches beyond the cap — but we stopped
                        // looking, so this counts as truncated too, not
                        // just the global cap.
                        truncated = true;
                        break;
                    }
                    if matches.len() >= MAX_TOTAL_MATCHES {
                        truncated = true;
                        break 'repos;
                    }
                }
            }
        }
    }

    SearchReport {
        matches,
        repositories_searched,
        truncated,
    }
}
