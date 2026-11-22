//! `.gitignore` generation for the `CreateGitignore` fix.
//!
//! The template is assembled from whichever detected technologies apply
//! (spec section 7's detection results feed straight in) plus a small set
//! of universal entries. Nothing here is guessed for a stack that wasn't
//! actually detected.

use crate::types::TechDetection;
use std::path::Path;

const UNIVERSAL: &[&str] = &[".DS_Store", "*.log", ".env", ".env.local"];

/// Append one ignore pattern to a repository's `.gitignore` (spec section
/// 12's "Add pattern to .gitignore" action for security findings).
///
/// Creates `.gitignore` when missing. Refuses when the exact pattern is
/// already present (as a trimmed full-line match), when the pattern is
/// blank, contains a newline, or tries to escape with `..` — gitignore
/// patterns are repo-relative, so `..` is never meaningful here. Returns
/// the file's new full content so callers can preview what was written.
pub fn append_pattern(root: &Path, pattern: &str) -> Result<String, String> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err("Ignore pattern must not be blank.".to_string());
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("Ignore pattern must be a single line.".to_string());
    }
    if trimmed.split('/').any(|seg| seg == "..") {
        return Err("Ignore pattern must stay inside the repository — `..` is not allowed.".to_string());
    }
    if trimmed.len() > 256 {
        return Err("Ignore pattern is too long (max 256 characters).".to_string());
    }

    let target = root.join(".gitignore");
    let existing = std::fs::read_to_string(&target).unwrap_or_default();
    let already = existing
        .lines()
        .any(|line| line.trim() == trimmed);
    if already {
        return Err(format!("`{trimmed}` is already in .gitignore."));
    }

    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(trimmed);
    out.push('\n');

    std::fs::write(&target, &out).map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn generate(techs: &[TechDetection]) -> String {
    let names: Vec<&str> = techs.iter().map(|t| t.name.as_str()).collect();
    let has = |n: &str| names.contains(&n);

    let mut sections: Vec<(&str, Vec<&str>)> = Vec::new();

    if has("Node.js") {
        sections.push((
            "Node.js",
            vec!["node_modules/", "dist/", "build/", "*.tsbuildinfo"],
        ));
    }
    if has("Rust") {
        sections.push(("Rust", vec!["/target", "**/*.rs.bk"]));
    }
    if has("Python") {
        sections.push((
            "Python",
            vec![
                "__pycache__/",
                "*.pyc",
                ".venv/",
                "venv/",
                "*.egg-info/",
                ".pytest_cache/",
            ],
        ));
    }
    if has("Go") {
        sections.push(("Go", vec!["/bin", "/vendor"]));
    }
    if has("Dart") || has("Flutter") {
        sections.push((
            "Dart/Flutter",
            vec![".dart_tool/", ".flutter-plugins", "build/"],
        ));
    }
    if has("Ruby") {
        sections.push(("Ruby", vec![".bundle/", "vendor/bundle/"]));
    }
    if has("Java") || has("Gradle") {
        sections.push(("JVM", vec!["target/", "build/", "*.class"]));
    }
    if has("Tauri") {
        sections.push(("Tauri", vec!["src-tauri/target/", "dist/"]));
    }

    let mut out = String::new();
    for (label, entries) in &sections {
        out.push_str(&format!("# {label}\n"));
        for e in entries {
            out.push_str(e);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("# General\n");
    for e in UNIVERSAL {
        out.push_str(e);
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TechDetection;

    fn tech(name: &str) -> TechDetection {
        TechDetection {
            name: name.to_string(),
            evidence: "test".to_string(),
        }
    }

    #[test]
    fn includes_node_modules_only_when_node_detected() {
        let with_node = generate(&[tech("Node.js")]);
        assert!(with_node.contains("node_modules/"));

        let without_node = generate(&[tech("Go")]);
        assert!(!without_node.contains("node_modules/"));
    }

    #[test]
    fn combines_sections_for_multiple_detected_stacks() {
        let out = generate(&[tech("Rust"), tech("Tauri"), tech("Node.js")]);
        assert!(out.contains("/target"));
        assert!(out.contains("src-tauri/target/"));
        assert!(out.contains("node_modules/"));
    }

    #[test]
    fn always_includes_universal_entries() {
        let out = generate(&[]);
        assert!(out.contains(".DS_Store"));
        assert!(out.contains(".env"));
    }

    #[test]
    fn append_creates_gitignore_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let out = append_pattern(tmp.path(), ".env.local").unwrap();
        assert!(out.contains(".env.local"));
        let on_disk = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(on_disk, out);
    }

    #[test]
    fn append_adds_to_existing_file_without_clobbering() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "node_modules/\n").unwrap();
        let out = append_pattern(tmp.path(), "secrets.env").unwrap();
        assert!(out.contains("node_modules/"));
        assert!(out.contains("secrets.env"));
    }

    #[test]
    fn append_refuses_a_duplicate_pattern() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), ".env\n").unwrap();
        let err = append_pattern(tmp.path(), " .env ").unwrap_err();
        assert!(err.contains("already in .gitignore"));
    }

    #[test]
    fn append_rejects_blank_newline_and_parent_escape() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(append_pattern(tmp.path(), "   ").is_err());
        assert!(append_pattern(tmp.path(), "a\nb").is_err());
        assert!(append_pattern(tmp.path(), "../outside").is_err());
    }
}
