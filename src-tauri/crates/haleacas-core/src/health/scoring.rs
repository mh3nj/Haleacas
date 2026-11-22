//! The "Haleacas Health Score" (spec section 9).
//!
//! Deliberately presented as *Haleacas'* scoring, not an objective
//! universal standard — every number here traces back to a concrete,
//! already-collected fact (README presence, `.gitignore` presence, etc.),
//! never a guess. Categories the spec describes but that need data we
//! don't collect yet (Dependencies, Security, Maintainability) are listed
//! in `not_yet_scored` rather than silently scored on placeholder logic.

use crate::types::{CategoryScore, HealthCheck, HealthReport, RepositoryFingerprint};

pub fn score(fp: &RepositoryFingerprint) -> HealthReport {
    let categories = vec![
        documentation(fp),
        structure(fp),
        testing(fp),
        git_hygiene(fp),
        configuration(fp),
        security(fp),
    ];

    let overall = if categories.is_empty() {
        0
    } else {
        let sum: u32 = categories.iter().map(|c| c.score as u32).sum();
        (sum / categories.len() as u32) as u8
    };

    HealthReport {
        overall,
        categories,
        not_yet_scored: vec!["Dependencies".to_string(), "Maintainability".to_string()],
    }
}

/// Turns a set of weighted pass/fail checks into a 0–100 score. Shared
/// by every health category here, and by README Studio's quality
/// analyzer (spec section 17) — same "traceable to concrete checks,
/// never a bare number" principle applies to both.
pub(crate) fn category_score(category: &str, checks: Vec<HealthCheck>) -> CategoryScore {
    let total_weight: u32 = checks.iter().map(|c| c.weight).sum();
    let earned_weight: u32 = checks
        .iter()
        .filter(|c| c.passed)
        .map(|c| c.weight)
        .sum();
    let score = if total_weight == 0 {
        0
    } else {
        ((earned_weight as f64 / total_weight as f64) * 100.0).round() as u8
    };

    CategoryScore {
        category: category.to_string(),
        score,
        checks,
    }
}

fn documentation(fp: &RepositoryFingerprint) -> CategoryScore {
    category_score(
        "Documentation",
        vec![
            HealthCheck {
                label: "README exists".to_string(),
                passed: fp.has_readme,
                detail: if fp.has_readme {
                    "A README file was found at the repository root.".to_string()
                } else {
                    "No README.md (or similarly named file) was found.".to_string()
                },
                weight: 3,
            },
            HealthCheck {
                label: "License present".to_string(),
                passed: fp.license.is_some(),
                detail: match &fp.license {
                    Some(l) => format!("A LICENSE file was found and identified as {l}."),
                    None => "No LICENSE file was found at the repository root.".to_string(),
                },
                weight: 2,
            },
        ],
    )
}

fn structure(fp: &RepositoryFingerprint) -> CategoryScore {
    category_score(
        "Structure",
        vec![
            HealthCheck {
                label: ".gitignore present".to_string(),
                passed: fp.has_gitignore,
                detail: if fp.has_gitignore {
                    "A .gitignore file was found.".to_string()
                } else {
                    "No .gitignore file was found.".to_string()
                },
                weight: 2,
            },
            HealthCheck {
                label: "Technology stack identifiable".to_string(),
                passed: !fp.techs.is_empty(),
                detail: if fp.techs.is_empty() {
                    "No recognized manifest (package.json, Cargo.toml, etc.) was found."
                        .to_string()
                } else {
                    format!(
                        "Detected: {}.",
                        fp.techs
                            .iter()
                            .map(|t| t.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                },
                weight: 1,
            },
        ],
    )
}

fn testing(fp: &RepositoryFingerprint) -> CategoryScore {
    category_score(
        "Testing",
        vec![HealthCheck {
            label: "Test directory detected".to_string(),
            passed: fp.has_tests,
            detail: if fp.has_tests {
                "A tests/, test/, __tests__/, or spec/ directory was found.".to_string()
            } else {
                "No conventional test directory was found. This does not rule out inline tests \
                 Haleacas doesn't yet recognize."
                    .to_string()
            },
            weight: 1,
        }],
    )
}

fn git_hygiene(fp: &RepositoryFingerprint) -> CategoryScore {
    category_score(
        "Git hygiene",
        vec![
            HealthCheck {
                label: "Working tree is clean".to_string(),
                passed: !fp.git.is_dirty,
                detail: if fp.git.is_dirty {
                    format!(
                        "{} file(s) have uncommitted changes.",
                        fp.git.changed_files.len()
                    )
                } else {
                    "No uncommitted changes.".to_string()
                },
                weight: 2,
            },
            HealthCheck {
                label: "Has commit history".to_string(),
                passed: fp.git.commit_count > 0,
                detail: format!("{} commit(s) found on the current branch.", fp.git.commit_count),
                weight: 1,
            },
        ],
    )
}

fn configuration(fp: &RepositoryFingerprint) -> CategoryScore {
    category_score(
        "Configuration",
        vec![HealthCheck {
            label: "CI configuration present".to_string(),
            passed: fp.has_ci,
            detail: if fp.has_ci {
                "A CI configuration (.github/workflows, .gitlab-ci.yml, or .circleci) was found."
                    .to_string()
            } else {
                "No CI configuration was found.".to_string()
            },
            weight: 1,
        }],
    )
}

fn security(fp: &RepositoryFingerprint) -> CategoryScore {
    category_score(
        "Security",
        vec![HealthCheck {
            label: "No potential secrets detected".to_string(),
            passed: fp.security_findings.is_empty(),
            detail: if fp.security_findings.is_empty() {
                "Haleacas' lightweight heuristic scanner found nothing credential-shaped. This \
                 is not a guarantee — it only checks for a handful of distinctive patterns."
                    .to_string()
            } else {
                format!(
                    "{} potential secret(s) flagged by the heuristic scanner. Verify each \
                     manually — false positives are expected.",
                    fp.security_findings.len()
                )
            },
            weight: 1,
        }],
    )
}
