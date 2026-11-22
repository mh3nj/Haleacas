//! Repository Doctor (spec section 10).
//!
//! Every finding here traces back to a signal already collected in the
//! fingerprint — nothing is invented, and nothing requires re-scanning the
//! filesystem. `Critical` severity fires when the lightweight secret
//! scanner (section 12) flags something — see `security::scan`.

use crate::types::{DoctorFinding, FindingSeverity, RepositoryFingerprint};

pub fn diagnose(fp: &RepositoryFingerprint) -> Vec<DoctorFinding> {
    let mut findings = Vec::new();

    if !fp.security_findings.is_empty() {
        let mut affected: Vec<String> = fp
            .security_findings
            .iter()
            .map(|f| f.file.clone())
            .collect();
        affected.sort();
        affected.dedup();

        findings.push(DoctorFinding {
            severity: FindingSeverity::Critical,
            title: format!(
                "{} possible secret(s) detected",
                fp.security_findings.len()
            ),
            explanation: "Haleacas' lightweight heuristic scanner flagged text that looks like \
                          it could be a credential (API key, access token, private key, etc.)."
                .to_string(),
            why_it_matters: "If any of these are real credentials, anyone with access to this \
                              repository (or its Git history, even after the file is later \
                              edited) can use them."
                .to_string(),
            recommended_action: "Review each flagged location. This is a heuristic scanner, not \
                                  a guarantee — verify manually, since false positives are \
                                  expected. If a match is a real credential, rotate it \
                                  immediately and remove it from the repository (including its \
                                  Git history, which a normal commit does not do)."
                .to_string(),
            affected_files: affected.into_iter().take(10).collect(),
        });
    }

    if !fp.has_readme {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            title: "No README detected".to_string(),
            explanation: "No README.md (or similarly named file) was found at the repository \
                          root."
                .to_string(),
            why_it_matters: "A README is usually the first thing anyone — including you in six \
                              months — reads to understand what a project is and how to run it."
                .to_string(),
            recommended_action: "Add a README.md describing what the project is, how to install \
                                  it, and how to use it."
                .to_string(),
            affected_files: vec!["README.md".to_string()],
        });
    }

    if fp.license.is_none() {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            title: "No license detected".to_string(),
            explanation: "No LICENSE file was found at the repository root.".to_string(),
            why_it_matters: "Without a license, the default legal position in most \
                              jurisdictions is that no one else has permission to use, copy, or \
                              modify the code — even if the repository is public."
                .to_string(),
            recommended_action: "Add a LICENSE file appropriate for how you want the project \
                                  used."
                .to_string(),
            affected_files: vec!["LICENSE".to_string()],
        });
    }

    if !fp.has_gitignore {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            title: "No .gitignore detected".to_string(),
            explanation: "No .gitignore file was found.".to_string(),
            why_it_matters: "Without one, build output, dependency folders, and local config \
                              are easy to accidentally commit."
                .to_string(),
            recommended_action: "Add a .gitignore appropriate for the detected tech stack."
                .to_string(),
            affected_files: vec![".gitignore".to_string()],
        });
    }

    if !fp.has_tests {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Improvement,
            title: "No automated tests detected".to_string(),
            explanation: "No conventional test directory (tests/, test/, __tests__/, spec/) was \
                          found."
                .to_string(),
            why_it_matters: "This doesn't necessarily mean there are no tests — Haleacas only \
                              recognizes a few conventional layouts — but if there genuinely \
                              aren't any, regressions are easy to miss."
                .to_string(),
            recommended_action: "Add a test directory and at least a basic test suite, or check \
                                  whether tests live somewhere Haleacas doesn't yet recognize."
                .to_string(),
            affected_files: vec![],
        });
    }

    if !fp.has_ci {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Info,
            title: "No CI configuration detected".to_string(),
            explanation: "No .github/workflows, .gitlab-ci.yml, or .circleci configuration was \
                          found."
                .to_string(),
            why_it_matters: "Without CI, tests and checks only run when someone remembers to \
                              run them locally."
                .to_string(),
            recommended_action: "Consider adding a CI workflow that runs your tests on every \
                                  push."
                .to_string(),
            affected_files: vec![".github/workflows".to_string()],
        });
    }

    if fp.git.is_dirty {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Info,
            title: "Uncommitted changes present".to_string(),
            explanation: format!(
                "{} file(s) have uncommitted changes as of the last scan.",
                fp.git.changed_files.len()
            ),
            why_it_matters: "Uncommitted work isn't backed up by Git and can be lost.".to_string(),
            recommended_action: "Commit, stash, or otherwise account for the pending changes."
                .to_string(),
            affected_files: fp
                .git
                .changed_files
                .iter()
                .take(10)
                .map(|c| c.path.clone())
                .collect(),
        });
    }

    if fp.techs.is_empty() {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Info,
            title: "Could not confidently detect a technology stack".to_string(),
            explanation: "None of Haleacas' recognized manifests (package.json, Cargo.toml, \
                          pyproject.toml, go.mod, pubspec.yaml, Gemfile, pom.xml, build.gradle) \
                          were found at the repository root."
                .to_string(),
            why_it_matters: "This is informational, not a problem — some projects genuinely \
                              don't have a manifest at the root, or use one Haleacas doesn't \
                              recognize yet."
                .to_string(),
            recommended_action: "No action needed unless this is unexpected for this project."
                .to_string(),
            affected_files: vec![],
        });
    }

    if fp.git.commit_count == 0 {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            title: "No commit history".to_string(),
            explanation: "This repository has no commits on the current branch.".to_string(),
            why_it_matters: "An empty history usually means the repository was just initialized \
                              and nothing has been committed yet."
                .to_string(),
            recommended_action: "Make an initial commit if this wasn't intentional.".to_string(),
            affected_files: vec![],
        });
    }

    findings
}
