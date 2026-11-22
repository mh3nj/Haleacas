//! Repository Explorer (spec section 15).
//!
//! A visual project explorer that goes beyond a bare file tree with a bit
//! of metadata per entry — but deliberately does *not* try to infer
//! architecture or file relationships that can't be reliably determined
//! (spec section 15: "clearly distinguish detected / inferred / unknown").
//! What's here is the "detected" tier only: real file sizes, real
//! modification times, and test/config classification against a small,
//! explicit, documented set of naming conventions — nothing guessed.

use crate::skip_dirs::is_skipped_dir_name;
use crate::types::{DirectoryListing, ExplorerHighlights, FileNode};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const CONFIG_FILENAMES: &[&str] = &[
    "package.json",
    "tsconfig.json",
    "Cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "go.mod",
    "pubspec.yaml",
    "Gemfile",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    ".eslintrc.json",
    ".eslintrc",
    ".prettierrc",
    ".gitignore",
    ".env.example",
    "Dockerfile",
    "docker-compose.yml",
    "tauri.conf.json",
    "vite.config.ts",
    "webpack.config.js",
];

const TEST_DIR_NAMES: &[&str] = &["test", "tests", "__tests__", "spec"];

/// Resolve `relative` against `root`, refusing anything that would escape
/// the repository (`..` segments, absolute paths, symlink escapes — spec
/// section 35: "sanitized paths"). `relative` may be empty for the root
/// itself.
fn resolve_within(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let candidate = if relative.is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    };

    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("Could not resolve repository root: {e}"))?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|e| format!("Path not found: {e}"))?;

    if !canonical_candidate.starts_with(&canonical_root) {
        return Err("Path is outside the repository.".to_string());
    }

    Ok(canonical_candidate)
}

pub fn list_directory(root: &Path, relative: &str) -> Result<DirectoryListing, String> {
    let dir = resolve_within(root, relative)?;
    if !dir.is_dir() {
        return Err("Not a directory.".to_string());
    }

    let mut entries = Vec::new();
    let read_dir = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in read_dir {
        let Ok(entry) = entry else { continue };
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let entry_relative = to_relative(root, &entry.path());

        entries.push(FileNode {
            name: name.clone(),
            relative_path: entry_relative.clone(),
            is_dir: meta.is_dir(),
            size_bytes: if meta.is_dir() { 0 } else { meta.len() },
            modified_at: modified_at_rfc3339(&meta),
            is_test: is_test_path(&entry_relative, &name),
            is_config: !meta.is_dir() && CONFIG_FILENAMES.contains(&name.as_str()),
        });
    }

    // Directories first, then alphabetical within each group — a
    // consistent, predictable order rather than raw filesystem order.
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));

    Ok(DirectoryListing {
        relative_path: relative.to_string(),
        entries,
    })
}

const HIGHLIGHT_COUNT: usize = 10;

pub fn highlights(root: &Path) -> ExplorerHighlights {
    let mut files = Vec::new();

    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        if !entry.file_type().is_dir() {
            return true;
        }
        let name = entry.file_name().to_string_lossy();
        !is_skipped_dir_name(&name)
    });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let relative = to_relative(root, entry.path());

        files.push(FileNode {
            name: name.clone(),
            relative_path: relative.clone(),
            is_dir: false,
            size_bytes: meta.len(),
            modified_at: modified_at_rfc3339(&meta),
            is_test: is_test_path(&relative, &name),
            is_config: CONFIG_FILENAMES.contains(&name.as_str()),
        });
    }

    let mut largest_files = files.clone();
    largest_files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    largest_files.truncate(HIGHLIGHT_COUNT);

    let mut recently_modified = files;
    recently_modified.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    recently_modified.truncate(HIGHLIGHT_COUNT);

    ExplorerHighlights {
        largest_files,
        recently_modified,
    }
}

/// Per-file cap for `read_file_for_context` — large enough for any real
/// README/source file a "Selected files" AI context would reasonably
/// include, small enough that one oversized file can't blow up a
/// prompt. Applied on the *string* (post-UTF8-decode), never on raw
/// bytes, so truncation can never land mid-character.
const AI_CONTEXT_FILE_CHAR_CAP: usize = 40_000;

/// Reads one file's text content for use as AI context (spec section
/// 18's "Selected files"). Reuses `resolve_within`'s path-escape guard
/// — the same protection `list_directory` gets — so a selected "file"
/// can never resolve outside the repository root. Refuses anything
/// that isn't valid UTF-8 text rather than silently sending binary
/// bytes to an AI provider, and caps size rather than sending an
/// unbounded amount of file content into a prompt.
pub fn read_file_for_context(root: &Path, relative: &str) -> Result<(String, bool), String> {
    let path = resolve_within(root, relative)?;
    if path.is_dir() {
        return Err(format!("{relative} is a directory, not a file."));
    }

    let full = fs::read_to_string(&path).map_err(|_| {
        format!("{relative} could not be read as text — it may be binary, or no longer exists.")
    })?;

    Ok(truncate_str_chars(&full, AI_CONTEXT_FILE_CHAR_CAP))
}

/// Truncates `s` to at most `max_chars` *characters* (not bytes), so a
/// multi-byte UTF-8 character is never split mid-sequence. Returns
/// whether truncation actually happened.
fn truncate_str_chars(s: &str, max_chars: usize) -> (String, bool) {
    if s.chars().count() <= max_chars {
        return (s.to_string(), false);
    }
    (s.chars().take(max_chars).collect(), true)
}

fn to_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn modified_at_rfc3339(meta: &fs::Metadata) -> Option<String> {
    meta.modified()
        .ok()
        .map(DateTime::<Utc>::from)
        .map(|dt| dt.to_rfc3339())
}

/// Test-file/directory detection by naming convention only — never a
/// guess about what a file's *content* does. Matches: a path that
/// passes through a directory named test/tests/__tests__/spec, or a
/// filename following the `*.test.*` / `*.spec.*` / `test_*` / `*_test.*`
/// conventions.
fn is_test_path(relative_path: &str, file_name: &str) -> bool {
    let in_test_dir = relative_path
        .split('/')
        .any(|segment| TEST_DIR_NAMES.contains(&segment.to_lowercase().as_str()));
    if in_test_dir {
        return true;
    }

    let lower = file_name.to_lowercase();
    let stem_pattern = lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.starts_with("test_")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.py");

    stem_pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_test_directory_membership() {
        assert!(is_test_path("tests/basic.rs", "basic.rs"));
        assert!(is_test_path("src/__tests__/App.test.tsx", "App.test.tsx"));
        assert!(!is_test_path("src/app.tsx", "app.tsx"));
    }

    #[test]
    fn detects_test_filename_conventions() {
        assert!(is_test_path("src/util.test.ts", "util.test.ts"));
        assert!(is_test_path("src/util.spec.ts", "util.spec.ts"));
        assert!(is_test_path("test_util.py", "test_util.py"));
        assert!(is_test_path("util_test.go", "util_test.go"));
        assert!(!is_test_path("src/contest.ts", "contest.ts"));
    }

    #[test]
    fn truncate_str_chars_leaves_short_strings_untouched() {
        let (out, truncated) = truncate_str_chars("hello", 100);
        assert_eq!(out, "hello");
        assert!(!truncated);
    }

    #[test]
    fn truncate_str_chars_never_splits_a_multibyte_character() {
        let input = "é".repeat(50);
        let (out, truncated) = truncate_str_chars(&input, 10);
        assert!(truncated);
        assert_eq!(out, "é".repeat(10));
    }

    #[test]
    fn read_file_for_context_reads_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "# Hello\n\nSome content.").unwrap();

        let (content, truncated) = read_file_for_context(dir.path(), "README.md").unwrap();
        assert_eq!(content, "# Hello\n\nSome content.");
        assert!(!truncated);
    }

    #[test]
    fn read_file_for_context_flags_truncation_over_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let big = "x".repeat(AI_CONTEXT_FILE_CHAR_CAP + 500);
        std::fs::write(dir.path().join("big.txt"), &big).unwrap();

        let (content, truncated) = read_file_for_context(dir.path(), "big.txt").unwrap();
        assert!(truncated);
        assert_eq!(content.chars().count(), AI_CONTEXT_FILE_CHAR_CAP);
    }

    #[test]
    fn read_file_for_context_refuses_a_path_escaping_the_repository() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("repo")).unwrap();
        std::fs::write(dir.path().join("secret.txt"), "outside the repo").unwrap();

        let result = read_file_for_context(&dir.path().join("repo"), "../secret.txt");
        assert!(result.is_err());
    }

    #[test]
    fn read_file_for_context_refuses_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();

        let result = read_file_for_context(dir.path(), "src");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("directory"));
    }

    #[test]
    fn read_file_for_context_refuses_binary_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("image.png"), [0xFFu8, 0xD8, 0x00, 0xFE, 0x12, 0x00]).unwrap();

        let result = read_file_for_context(dir.path(), "image.png");
        assert!(result.is_err());
    }
}
