//! Basic README skeleton generation for the `CreateReadmeSkeleton` fix
//! (spec section 11: "Generate basic README structure").
//!
//! This is deliberately minimal — the full authoring environment is
//! README Studio (Phase 5, not built yet). This just gives a Doctor
//! finding's "missing README" something concrete and honest to create:
//! real detected facts filled in, and an explicit placeholder wherever a
//! command or detail can't be confidently detected (spec section 16: "If
//! a command cannot be confidently detected, mark it as requiring user
//! input" — the same rule applies here even outside the full Studio).

use crate::types::RepositoryFingerprint;

pub fn generate(fp: &RepositoryFingerprint) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", fp.name));

    match &fp.description {
        Some(d) => out.push_str(&format!("{d}\n\n")),
        None => out.push_str("_TODO: add a one-line description of what this project does._\n\n"),
    }

    if !fp.techs.is_empty() {
        let stack = fp
            .techs
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(" · ");
        out.push_str(&format!("**Stack:** {stack}\n\n"));
    }

    out.push_str("## Installation\n\n");
    match install_command(fp) {
        Some(cmd) => out.push_str(&format!("```bash\n{cmd}\n```\n\n")),
        None => out.push_str("_TODO: add installation steps — Haleacas couldn't confidently \
                               detect one from the project's manifests._\n\n"),
    }

    out.push_str("## Usage\n\n_TODO: add a usage example._\n\n");

    out.push_str("## License\n\n");
    match &fp.license {
        Some(l) => out.push_str(&format!("{l} — see [LICENSE](./LICENSE).\n")),
        None => out.push_str("_TODO: add a LICENSE file and reference it here._\n"),
    }

    out
}

/// Only returns a command when a specific, unambiguous package manager
/// was actually detected — never a guess. `pub(crate)` so README Studio
/// (spec section 16) can reuse the exact same detection instead of
/// re-implementing it.
pub(crate) fn install_command(fp: &RepositoryFingerprint) -> Option<&'static str> {
    let has = |n: &str| fp.techs.iter().any(|t| t.name == n);

    if has("pnpm") {
        Some("pnpm install")
    } else if has("Yarn") {
        Some("yarn install")
    } else if has("npm") || has("Node.js") {
        Some("npm install")
    } else if has("Rust") {
        Some("cargo build")
    } else if has("Go") {
        Some("go build ./...")
    } else if has("Python") {
        Some("pip install -r requirements.txt")
    } else if has("Flutter") {
        Some("flutter pub get")
    } else if has("Ruby") {
        Some("bundle install")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GitInfo, HealthReport, TechDetection};

    fn base_fp() -> RepositoryFingerprint {
        RepositoryFingerprint {
            id: "abc".to_string(),
            name: "demo".to_string(),
            path: "/repos/demo".to_string(),
            description: None,
            techs: vec![],
            file_count: 0,
            approx_loc: 0,
            git: GitInfo::default(),
            license: None,
            has_readme: false,
            has_gitignore: false,
            has_ci: false,
            has_tests: false,
            size_bytes: 0,
            scanned_at: "2026-01-01T00:00:00Z".to_string(),
            health: HealthReport::default(),
            security_findings: vec![],
            security_findings_truncated: false,
        }
    }

    #[test]
    fn uses_real_description_when_present() {
        let mut fp = base_fp();
        fp.description = Some("A theme builder".to_string());
        let readme = generate(&fp);
        assert!(readme.contains("A theme builder"));
        assert!(!readme.contains("TODO: add a one-line description"));
    }

    #[test]
    fn placeholders_missing_description_rather_than_inventing_one() {
        let readme = generate(&base_fp());
        assert!(readme.contains("TODO: add a one-line description"));
    }

    #[test]
    fn picks_npm_install_for_a_plain_node_project() {
        let mut fp = base_fp();
        fp.techs = vec![TechDetection {
            name: "Node.js".to_string(),
            evidence: "package.json present".to_string(),
        }];
        let readme = generate(&fp);
        assert!(readme.contains("npm install"));
    }

    #[test]
    fn prefers_pnpm_over_npm_when_both_detected() {
        let mut fp = base_fp();
        fp.techs = vec![
            TechDetection {
                name: "Node.js".to_string(),
                evidence: "package.json present".to_string(),
            },
            TechDetection {
                name: "pnpm".to_string(),
                evidence: "pnpm-lock.yaml present".to_string(),
            },
        ];
        let readme = generate(&fp);
        assert!(readme.contains("```bash\npnpm install\n```"));
    }

    #[test]
    fn no_recognized_manifest_yields_a_placeholder_not_a_guess() {
        let readme = generate(&base_fp());
        assert!(readme.contains("couldn't confidently detect"));
    }

    #[test]
    fn references_detected_license_file() {
        let mut fp = base_fp();
        fp.license = Some("MIT".to_string());
        let readme = generate(&fp);
        assert!(readme.contains("MIT"));
        assert!(readme.contains("LICENSE"));
    }
}
