//! Lightweight heuristic security scanning (spec section 12).
//!
//! Explicitly **not** a substitute for a real security scanner — this
//! looks for a handful of distinctive credential-shaped patterns and
//! nothing more. False positives are expected and acknowledged, which is
//! exactly why every `SecurityFinding` reports only a file, a line
//! number, and a generic reason — **never the matched text itself**. A
//! real credential should never end up copied into Haleacas' own
//! database or UI just because Haleacas went looking for one.
//!
//! Repository contents are never uploaded anywhere; this is a purely
//! local, offline check (spec section 29).

use crate::skip_dirs::is_skipped_dir_name;
use crate::types::{IgnoredFinding, SecurityFinding};
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Cap on how many findings a single scan reports. A display/safety
/// limit only — scanning stops early once it's hit, which also bounds
/// how long a scan of a huge, secret-riddled repository can take.
const MAX_FINDINGS: usize = 50;

/// Skip files larger than this — large files are overwhelmingly data
/// dumps or binaries, not source containing hand-typed credentials, and
/// reading them line-by-line would be wasteful.
const MAX_FILE_SIZE_BYTES: u64 = 2 * 1024 * 1024;

/// Lockfiles are excluded: they're full of long hash-like strings that
/// are prime false-positive material for the generic patterns below, and
/// hand-typed credentials don't end up in them.
const SKIP_FILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "Gemfile.lock",
    "poetry.lock",
    "composer.lock",
];

struct Pattern {
    reason: &'static str,
    regex: Regex,
}

fn patterns() -> Vec<Pattern> {
    let compile = |reason: &'static str, pattern: &str| Pattern {
        reason,
        regex: Regex::new(pattern).expect("hardcoded security pattern must compile"),
    };

    vec![
        compile("Possible AWS access key ID", r"AKIA[0-9A-Z]{16}"),
        compile(
            "Possible AWS secret access key",
            r#"(?i)aws_secret_access_key\s*[:=]\s*['"][A-Za-z0-9/+=]{40}['"]"#,
        ),
        compile("Possible GitHub access token", r"gh[pousr]_[A-Za-z0-9]{36,}"),
        compile("Possible Slack token", r"xox[baprs]-[0-9A-Za-z-]{10,48}"),
        compile("Possible Google API key", r"AIza[0-9A-Za-z\-_]{35}"),
        compile("Possible Stripe live secret key", r"sk_live_[0-9a-zA-Z]{24,}"),
        compile(
            "Possible private key",
            r"-----BEGIN ([A-Z ]*)?PRIVATE KEY-----",
        ),
        compile(
            "Possible API key or token assignment",
            r#"(?i)\b(api[_-]?key|secret[_-]?key|access[_-]?token|auth[_-]?token)\b\s*[:=]\s*['"][A-Za-z0-9_\-]{20,}['"]"#,
        ),
    ]
}

pub struct ScanResult {
    pub findings: Vec<SecurityFinding>,
    pub truncated: bool,
}

pub fn scan(root: &Path) -> ScanResult {
    let patterns = patterns();
    let mut findings = Vec::new();
    let mut truncated = false;

    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        if !entry.file_type().is_dir() {
            return true;
        }
        let name = entry.file_name().to_string_lossy();
        !is_skipped_dir_name(&name)
    });

    'files: for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        if SKIP_FILES.contains(&file_name.as_ref()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.len() > MAX_FILE_SIZE_BYTES {
            continue;
        }

        // Skip binary files. `read_to_string` already refuses invalid
        // UTF-8, which covers the overwhelming majority of binaries.
        let Ok(contents) = fs::read_to_string(entry.path()) else {
            continue;
        };

        let relative = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        for (line_no, line) in contents.lines().enumerate() {
            for pattern in &patterns {
                if pattern.regex.is_match(line) {
                    findings.push(SecurityFinding {
                        file: relative.clone(),
                        line: line_no + 1,
                        reason: pattern.reason.to_string(),
                    });
                    if findings.len() >= MAX_FINDINGS {
                        truncated = true;
                        break 'files;
                    }
                    // Only report the first matching pattern per line —
                    // a line that trips two patterns doesn't need two
                    // findings.
                    break;
                }
            }
        }
    }

    ScanResult { findings, truncated }
}

/// Removes findings the user has already marked as ignored/false-positive
/// (spec section 12). Pure and cheap — matches purely on file + line +
/// reason, no filesystem access.
pub fn apply_ignore_list(
    findings: Vec<SecurityFinding>,
    ignored: &[IgnoredFinding],
) -> Vec<SecurityFinding> {
    findings
        .into_iter()
        .filter(|f| {
            !ignored
                .iter()
                .any(|ig| ig.file == f.file && ig.line == f.line && ig.reason == f.reason)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn findings_never_contain_the_matched_secret_text() {
        // A fake, obviously-not-real key used only to verify the scanner
        // extracts nothing but file/line/reason from it.
        let fake_key = "AKIAABCDEFGHIJKLMNOP";
        let patterns = patterns();
        let matched = patterns[0].regex.is_match(fake_key);
        assert!(matched, "fixture key should match the AWS key pattern");
        // The reason string is a static description, never built from
        // the input — this is really just documenting the invariant.
        assert!(!patterns[0].reason.contains(fake_key));
    }

    fn finding(file: &str, line: usize, reason: &str) -> SecurityFinding {
        SecurityFinding {
            file: file.to_string(),
            line,
            reason: reason.to_string(),
        }
    }

    fn ignored(file: &str, line: usize, reason: &str) -> IgnoredFinding {
        IgnoredFinding {
            file: file.to_string(),
            line,
            reason: reason.to_string(),
        }
    }

    #[test]
    fn ignore_list_removes_a_matching_finding() {
        let findings = vec![finding("a.py", 3, "Possible AWS access key ID")];
        let ignore_list = vec![ignored("a.py", 3, "Possible AWS access key ID")];

        let result = apply_ignore_list(findings, &ignore_list);

        assert!(result.is_empty());
    }

    #[test]
    fn ignore_list_only_removes_exact_file_line_reason_matches() {
        let findings = vec![
            finding("a.py", 3, "Possible AWS access key ID"),
            finding("a.py", 4, "Possible AWS access key ID"), // different line
            finding("b.py", 3, "Possible AWS access key ID"), // different file
            finding("a.py", 3, "Possible GitHub access token"), // different reason
        ];
        let ignore_list = vec![ignored("a.py", 3, "Possible AWS access key ID")];

        let result = apply_ignore_list(findings, &ignore_list);

        assert_eq!(result.len(), 3, "only the exact match should be removed");
    }

    #[test]
    fn empty_ignore_list_changes_nothing() {
        let findings = vec![finding("a.py", 3, "Possible AWS access key ID")];

        let result = apply_ignore_list(findings, &[]);

        assert_eq!(result.len(), 1);
    }
}
