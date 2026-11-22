//! Builds the text actually sent to the AI provider — never arbitrary
//! repository content, always a rendering of facts Haleacas already
//! collected (spec section 18: "send structured repository information
//! to the model instead of blindly sending the entire repository").
//!
//! Deliberately pure: every function here takes already-computed data
//! (`RepositoryFingerprint`, `DoctorFinding`s, a `DependencyReport`,
//! already-read file contents) and does no I/O and no network access
//! itself. The `app` crate's `commands::ai::ai_ask` is what actually
//! reads files from disk and calls `dependencies::scan` /
//! `health::doctor::diagnose` — keeping this module pure is what makes
//! it possible to unit-test the exact prompt content without a real
//! repository on disk.

use crate::types::{
    AIContext, AIContextSummary, Dependency, DependencyReport, DoctorFinding, RepositoryFingerprint,
};

/// Renders the "Repository Facts" baseline — included in every request
/// regardless of which `AIContext` variant was requested. Every line
/// traces back to a field already on `RepositoryFingerprint`,
/// `DoctorFinding`, or `DependencyReport` — nothing here is invented,
/// matching the same "never hallucinate" principle the rest of the app
/// already holds itself to for health scores and dependency data.
pub fn build_repository_facts(
    fp: &RepositoryFingerprint,
    findings: &[DoctorFinding],
    deps: &DependencyReport,
) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("# Repository: {}", fp.name));
    if let Some(desc) = &fp.description {
        lines.push(desc.clone());
    }
    lines.push(format!("Path: {}", fp.path));
    let techs: Vec<String> = fp.techs.iter().map(|t| t.name.clone()).collect();
    lines.push(format!(
        "Technologies detected: {}",
        if techs.is_empty() {
            "none detected".to_string()
        } else {
            techs.join(", ")
        }
    ));
    lines.push(format!("Approx. lines of code: {}", fp.approx_loc));
    lines.push(format!("File count: {}", fp.file_count));
    lines.push(format!(
        "License: {}",
        fp.license.as_deref().unwrap_or("not detected")
    ));
    lines.push(format!("README present: {}", fp.has_readme));
    lines.push(format!(".gitignore present: {}", fp.has_gitignore));
    lines.push(format!("CI configuration present: {}", fp.has_ci));
    lines.push(format!("Tests directory present: {}", fp.has_tests));
    if let Some(branch) = &fp.git.branch {
        lines.push(format!("Git branch: {branch}"));
    }
    lines.push(format!("Commit count: {}", fp.git.commit_count));
    if let Some(msg) = &fp.git.last_commit_message {
        lines.push(format!(
            "Last commit: {msg} ({})",
            fp.git.last_commit_author.as_deref().unwrap_or("unknown author")
        ));
    }

    lines.push(String::new());
    lines.push(format!(
        "## Haleacas Health Score: {}/100",
        fp.health.overall
    ));
    for cat in &fp.health.categories {
        lines.push(format!("- {}: {}/100", cat.category, cat.score));
    }
    if !fp.health.not_yet_scored.is_empty() {
        lines.push(format!(
            "- Not yet scored: {}",
            fp.health.not_yet_scored.join(", ")
        ));
    }

    lines.push(String::new());
    if findings.is_empty() {
        lines.push("## Repository Doctor: no findings.".to_string());
    } else {
        lines.push("## Repository Doctor findings:".to_string());
        for f in findings {
            lines.push(format!("- [{:?}] {}: {}", f.severity, f.title, f.explanation));
        }
    }

    lines.push(String::new());
    if deps.manifests.is_empty() {
        lines.push("## Dependencies: none detected.".to_string());
    } else {
        lines.push("## Dependencies:".to_string());
        for m in &deps.manifests {
            lines.push(format!(
                "- {} ({}): {} dependencies declared in {}",
                m.ecosystem,
                if m.lockfile_present {
                    "lockfile present"
                } else {
                    "no lockfile"
                },
                m.dependencies.len(),
                m.manifest_file
            ));
            // Named individually, not just counted — "explain my
            // dependencies" is a useless answer without actual names,
            // and every name here is a literal field from a real
            // parsed manifest (see `dependencies` module), never
            // resolved or freshness-checked.
            for dep in dep_sample(&m.dependencies) {
                lines.push(format!(
                    "  - {} {} ({:?})",
                    dep.name, dep.version_declared, dep.kind
                ));
            }
            if m.dependencies.len() > DEP_SAMPLE_CAP {
                lines.push(format!(
                    "  - ...and {} more not listed individually here",
                    m.dependencies.len() - DEP_SAMPLE_CAP
                ));
            }
        }
    }

    lines.join("\n")
}

/// Cap on how many individual dependencies get named per manifest in
/// the facts text — a repository with hundreds of transitive entries
/// shouldn't blow up the prompt size. The manifest-level summary line
/// above this is never capped, only the per-dependency listing.
const DEP_SAMPLE_CAP: usize = 25;

fn dep_sample(deps: &[Dependency]) -> &[Dependency] {
    if deps.len() > DEP_SAMPLE_CAP {
        &deps[..DEP_SAMPLE_CAP]
    } else {
        deps
    }
}

/// Per-file cap on how much of a selected file's content is quoted into
/// the prompt. Applied again here (on top of
/// `explorer::read_file_for_context`'s own cap) as a second bound
/// specifically on prompt size, since a future caller of this function
/// might not go through that path.
const FILE_QUOTE_CHAR_CAP: usize = 20_000;

/// Assembles the full context text for one AI request. `files` is
/// `(relative_path, content, was_truncated_on_disk_read)` — already
/// read by the caller (`app`'s `commands::ai`), since this function
/// does no I/O. Always includes the Repository Facts baseline; adds
/// selected file contents on top when `context` is `SelectedFiles`.
pub fn build_context(
    context: &AIContext,
    fp: &RepositoryFingerprint,
    findings: &[DoctorFinding],
    deps: &DependencyReport,
    files: &[(String, String, bool)],
) -> AIContextSummary {
    let facts = build_repository_facts(fp, findings, deps);

    match context {
        AIContext::RepositoryFacts => {
            let char_count = facts.chars().count();
            AIContextSummary {
                description:
                    "Repository facts only — name, detected tech, health score, Repository \
                     Doctor findings, dependency summary. No file contents."
                        .to_string(),
                text: facts,
                char_count,
                truncated: false,
            }
        }
        AIContext::SelectedFiles { paths } => {
            let mut text = facts;
            text.push_str("\n\n## Selected file contents:\n");
            let mut any_truncated = false;

            for (path, content, read_truncated) in files {
                let (quoted, quote_truncated) = truncate_at_char_boundary(content, FILE_QUOTE_CHAR_CAP);
                let truncated_here = *read_truncated || quote_truncated;
                if truncated_here {
                    any_truncated = true;
                }
                text.push_str(&format!(
                    "\n### {path}{}\n```\n{quoted}\n```\n",
                    if truncated_here { " (truncated)" } else { "" }
                ));
            }

            let char_count = text.chars().count();
            let names = if paths.is_empty() {
                "(none)".to_string()
            } else {
                paths.join(", ")
            };
            AIContextSummary {
                description: format!("Repository facts, plus selected file(s): {names}"),
                text,
                char_count,
                truncated: any_truncated,
            }
        }
    }
}

/// Truncates `s` to at most `max_chars` byte-safe (never splits a
/// multi-byte UTF-8 character), returning whether truncation happened.
/// Pulled out as its own pure function specifically so the
/// char-boundary edge case is directly testable in isolation.
fn truncate_at_char_boundary(s: &str, max_chars: usize) -> (String, bool) {
    if s.chars().count() <= max_chars {
        return (s.to_string(), false);
    }
    let truncated: String = s.chars().take(max_chars).collect();
    (truncated, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        CategoryScore, DependencyKind, DependencyManifest, FindingSeverity, GitInfo, HealthReport,
        SecurityFinding,
    };

    fn sample_fingerprint() -> RepositoryFingerprint {
        RepositoryFingerprint {
            id: "repo-1".to_string(),
            name: "Slate".to_string(),
            path: "/home/user/projects/slate".to_string(),
            description: Some("A Telegram theme builder.".to_string()),
            techs: vec![
                crate::types::TechDetection {
                    name: "React".to_string(),
                    evidence: "package.json dependency".to_string(),
                },
                crate::types::TechDetection {
                    name: "TypeScript".to_string(),
                    evidence: "tsconfig.json present".to_string(),
                },
            ],
            file_count: 120,
            approx_loc: 12_421,
            git: GitInfo {
                branch: Some("main".to_string()),
                remote_url: None,
                github_slug: None,
                is_dirty: false,
                ahead: 0,
                behind: 0,
                commit_count: 438,
                last_commit_hash: Some("abc123".to_string()),
                last_commit_message: Some("Fix theme export".to_string()),
                last_commit_author: Some("Jane Dev".to_string()),
                last_commit_at: Some("2026-08-01T10:00:00Z".to_string()),
                changed_files: vec![],
                changed_files_truncated: false,
            },
            license: Some("MIT".to_string()),
            has_readme: true,
            has_gitignore: true,
            has_ci: false,
            has_tests: true,
            size_bytes: 5_000_000,
            scanned_at: "2026-09-01T00:00:00Z".to_string(),
            health: HealthReport {
                overall: 87,
                categories: vec![CategoryScore {
                    category: "Documentation".to_string(),
                    score: 91,
                    checks: vec![],
                }],
                not_yet_scored: vec!["Maintainability".to_string()],
            },
            security_findings: vec![SecurityFinding {
                file: "config/example.env".to_string(),
                line: 3,
                reason: "Possible credential-like value detected.".to_string(),
            }],
            security_findings_truncated: false,
        }
    }

    fn sample_findings() -> Vec<DoctorFinding> {
        vec![DoctorFinding {
            severity: FindingSeverity::Warning,
            title: "No CI configuration detected".to_string(),
            explanation: "No .github/workflows was found.".to_string(),
            why_it_matters: "Tests only run locally.".to_string(),
            recommended_action: "Add a CI workflow.".to_string(),
            affected_files: vec![],
        }]
    }

    fn sample_deps() -> DependencyReport {
        DependencyReport {
            manifests: vec![DependencyManifest {
                ecosystem: "npm".to_string(),
                manifest_file: "package.json".to_string(),
                lockfile_present: true,
                lockfile_name: Some("package-lock.json".to_string()),
                dependencies: vec![
                    Dependency {
                        name: "react".to_string(),
                        version_declared: "^18.2.0".to_string(),
                        kind: DependencyKind::Direct,
                    },
                    Dependency {
                        name: "vite".to_string(),
                        version_declared: "^5.0.0".to_string(),
                        kind: DependencyKind::Dev,
                    },
                ],
            }],
        }
    }

    #[test]
    fn repository_facts_includes_real_fields_not_placeholders() {
        let text = build_repository_facts(&sample_fingerprint(), &sample_findings(), &sample_deps());
        assert!(text.contains("Slate"));
        assert!(text.contains("React"));
        assert!(text.contains("TypeScript"));
        assert!(text.contains("12421") || text.contains("12,421"));
        assert!(text.contains("87/100"));
        assert!(text.contains("No CI configuration detected"));
        assert!(text.contains("react"));
        assert!(text.contains("^18.2.0"));
        assert!(text.contains("vite"));
    }

    #[test]
    fn repository_facts_does_not_reach_into_raw_security_findings() {
        // `build_repository_facts` only takes Doctor findings, not the
        // fingerprint's raw `security_findings` — secret-related signal
        // reaches the AI only pre-summarized through a `DoctorFinding`
        // (e.g. "N possible secret(s) detected"), never as a raw
        // file+line match. The sample fixture's fingerprint has a
        // security finding pointing at "example.env" that is *not*
        // included in `sample_findings()`, so it should never appear.
        let text = build_repository_facts(&sample_fingerprint(), &sample_findings(), &sample_deps());
        assert!(!text.contains("example.env"));
    }

    #[test]
    fn empty_findings_says_so_plainly() {
        let text = build_repository_facts(&sample_fingerprint(), &[], &sample_deps());
        assert!(text.contains("Repository Doctor: no findings."));
    }

    #[test]
    fn empty_dependencies_says_so_plainly() {
        let text = build_repository_facts(&sample_fingerprint(), &sample_findings(), &DependencyReport::default());
        assert!(text.contains("Dependencies: none detected."));
    }

    #[test]
    fn dependency_sample_is_capped_and_says_how_many_more() {
        let many: Vec<Dependency> = (0..40)
            .map(|i| Dependency {
                name: format!("pkg-{i}"),
                version_declared: "1.0.0".to_string(),
                kind: DependencyKind::Direct,
            })
            .collect();
        let deps = DependencyReport {
            manifests: vec![DependencyManifest {
                ecosystem: "npm".to_string(),
                manifest_file: "package.json".to_string(),
                lockfile_present: true,
                lockfile_name: None,
                dependencies: many,
            }],
        };
        let text = build_repository_facts(&sample_fingerprint(), &[], &deps);
        assert!(text.contains("pkg-0"));
        assert!(text.contains("pkg-24"));
        assert!(!text.contains("pkg-25"));
        assert!(text.contains("...and 15 more not listed individually here"));
    }

    #[test]
    fn repository_facts_context_has_no_file_contents_and_is_not_truncated() {
        let summary = build_context(
            &AIContext::RepositoryFacts,
            &sample_fingerprint(),
            &sample_findings(),
            &sample_deps(),
            &[],
        );
        assert!(!summary.text.contains("Selected file contents"));
        assert!(!summary.truncated);
        assert_eq!(summary.char_count, summary.text.chars().count());
    }

    #[test]
    fn selected_files_context_includes_facts_and_file_content() {
        let context = AIContext::SelectedFiles {
            paths: vec!["README.md".to_string()],
        };
        let files = vec![("README.md".to_string(), "# Slate\n\nA theme builder.".to_string(), false)];
        let summary = build_context(&context, &sample_fingerprint(), &sample_findings(), &sample_deps(), &files);
        assert!(summary.text.contains("# Repository: Slate"));
        assert!(summary.text.contains("Selected file contents"));
        assert!(summary.text.contains("### README.md"));
        assert!(summary.text.contains("A theme builder."));
        assert!(!summary.truncated);
        assert!(summary.description.contains("README.md"));
    }

    #[test]
    fn selected_files_context_flags_truncation_from_disk_read() {
        let context = AIContext::SelectedFiles {
            paths: vec!["big.log".to_string()],
        };
        let files = vec![("big.log".to_string(), "line one".to_string(), true)];
        let summary = build_context(&context, &sample_fingerprint(), &sample_findings(), &sample_deps(), &files);
        assert!(summary.truncated);
        assert!(summary.text.contains("(truncated)"));
    }

    #[test]
    fn truncate_at_char_boundary_leaves_short_strings_untouched() {
        let (out, truncated) = truncate_at_char_boundary("hello", 100);
        assert_eq!(out, "hello");
        assert!(!truncated);
    }

    #[test]
    fn truncate_at_char_boundary_never_splits_a_multibyte_character() {
        // Each "é" is 2 UTF-8 bytes but 1 char — truncating at a raw
        // byte index here would panic on a naive `&s[..n]` slice.
        let input = "é".repeat(50);
        let (out, truncated) = truncate_at_char_boundary(&input, 10);
        assert!(truncated);
        assert_eq!(out.chars().count(), 10);
        // Must still be valid UTF-8 — this alone would panic before
        // reaching the assert if the boundary were wrong.
        assert_eq!(out, "é".repeat(10));
    }

    #[test]
    fn truncate_at_char_boundary_exact_length_is_not_truncated() {
        let (out, truncated) = truncate_at_char_boundary("abcde", 5);
        assert_eq!(out, "abcde");
        assert!(!truncated);
    }
}
