//! Approximate size/structure metrics for a repository (spec section 7).
//!
//! LOC is explicitly "approximate": we count non-blank lines in a fixed set
//! of recognized source extensions and skip the same noisy directories the
//! discovery pass skips. This is a real count of real files — not an
//! estimate pulled from a formula — but it is not a full tokenizer/AST-based
//! count, so we never present it as exact.

use crate::skip_dirs::is_skipped_dir_name;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "rb", "java", "kt", "swift", "dart", "c", "h",
    "cpp", "hpp", "cc", "cs", "php", "vue", "svelte", "css", "scss", "html",
];

pub struct StructuralMetrics {
    pub file_count: usize,
    pub approx_loc: usize,
    pub size_bytes: u64,
    pub has_readme: bool,
    pub has_gitignore: bool,
    pub has_ci: bool,
    pub has_tests: bool,
    pub license: Option<String>,
}

pub fn collect(root: &Path) -> StructuralMetrics {
    let mut file_count = 0usize;
    let mut approx_loc = 0usize;
    let mut size_bytes = 0u64;

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
        file_count += 1;
        if let Ok(meta) = entry.metadata() {
            size_bytes += meta.len();
        }

        let is_source = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| SOURCE_EXTENSIONS.contains(&e))
            .unwrap_or(false);

        if is_source {
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                approx_loc += contents.lines().filter(|l| !l.trim().is_empty()).count();
            }
        }
    }

    let has_readme = find_case_insensitive(root, &["readme.md", "readme", "readme.txt"]);
    let has_gitignore = root.join(".gitignore").is_file();
    let has_ci = root.join(".github/workflows").is_dir()
        || root.join(".gitlab-ci.yml").is_file()
        || root.join(".circleci").is_dir();
    let has_tests = root.join("tests").is_dir()
        || root.join("test").is_dir()
        || root.join("__tests__").is_dir()
        || root.join("spec").is_dir();

    let license = find_case_insensitive_path(root, &["license", "license.md", "license.txt"])
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|text| classify_license(&text));

    StructuralMetrics {
        file_count,
        approx_loc,
        size_bytes,
        has_readme,
        has_gitignore,
        has_ci,
        has_tests,
        license,
    }
}

fn find_case_insensitive(root: &Path, names: &[&str]) -> bool {
    find_case_insensitive_path(root, names).is_some()
}

fn find_case_insensitive_path(root: &Path, names: &[&str]) -> Option<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return None;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_lowercase();
        if names.contains(&file_name.as_str()) {
            return Some(entry.path());
        }
    }
    None
}

/// Very small heuristic license identifier based on a few common phrases.
/// This is not a substitute for an SPDX license scanner — it only recognizes
/// the handful of licenses whose canonical text is unambiguous.
fn classify_license(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.contains("mit license") {
        "MIT".to_string()
    } else if lower.contains("apache license") {
        "Apache-2.0".to_string()
    } else if lower.contains("gnu general public license") && lower.contains("version 3") {
        "GPL-3.0".to_string()
    } else if lower.contains("gnu general public license") {
        "GPL".to_string()
    } else if lower.contains("bsd") {
        "BSD".to_string()
    } else if lower.contains("mozilla public license") {
        "MPL-2.0".to_string()
    } else {
        "Unrecognized (present, not identified)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_mit() {
        assert_eq!(classify_license("MIT License\n\nCopyright..."), "MIT");
    }

    #[test]
    fn classifies_apache() {
        assert_eq!(
            classify_license("Apache License\nVersion 2.0"),
            "Apache-2.0"
        );
    }

    #[test]
    fn classifies_gpl3() {
        assert_eq!(
            classify_license("GNU GENERAL PUBLIC LICENSE\nVersion 3"),
            "GPL-3.0"
        );
    }

    #[test]
    fn classifies_unrecognized_text_without_guessing() {
        assert_eq!(
            classify_license("All rights reserved. Do not copy."),
            "Unrecognized (present, not identified)"
        );
    }
}
