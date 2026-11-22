//! README Studio (spec section 16) — a structured, section-based README
//! authoring environment, distinct from (and reusing facts the same way
//! as) the one-click "Generate basic README" Doctor fix in
//! `fixes::readme`. That fix produces one flat string for a quick Doctor
//! action; this module produces an ordered, editable `Vec<ReadmeSection>`
//! that the frontend can reorder, add to, and remove from before
//! rendering back to Markdown — see `quality` for the separate spec
//! section 17 analyzer that scores an *existing* README's text.
//!
//! Deliberately scoped as "compose a README from repository facts," not
//! "parse an arbitrary existing README.md back into these sections."
//! Round-tripping arbitrary existing Markdown into a fixed section-kind
//! catalog would mean guessing which of a repo's own headings map to
//! which named kind — exactly the kind of shallow, unreliable inference
//! this project avoids elsewhere. Opening the Studio on a repository
//! that already has a README simply starts a new composition from the
//! same real facts; the existing file is never read or overwritten
//! until the user explicitly saves.

use crate::fixes::readme::install_command;
use crate::types::{ReadmeCatalogEntry, ReadmeSection, ReadmeSectionKind, RepositoryFingerprint};

pub mod quality;

/// The starter set a freshly-opened Studio seeds itself with — the same
/// four facts the one-click "Generate basic README" Doctor fix already
/// fills in (Hero/Installation/Usage/License), just as separately
/// editable, reorderable sections instead of one flat string. The other
/// 12 catalog kinds (Features, Screenshots, ...) aren't things a
/// repository scan can honestly generate content for, so they're only
/// ever added by the user explicitly via the "Add Section" catalog
/// rather than appearing here as empty placeholders nobody asked for.
pub fn generate_sections(fp: &RepositoryFingerprint) -> Vec<ReadmeSection> {
    vec![
        ReadmeSection {
            id: "hero".to_string(),
            kind: ReadmeSectionKind::Hero,
            title: fp.name.clone(),
            content: fp.description.clone().unwrap_or_else(|| {
                "_TODO: add a one-line tagline for what this project does._".to_string()
            }),
            auto_generated: fp.description.is_some(),
        },
        ReadmeSection {
            id: "installation".to_string(),
            kind: ReadmeSectionKind::Installation,
            title: "Installation".to_string(),
            content: match install_command(fp) {
                Some(cmd) => format!("```bash\n{cmd}\n```"),
                None => "_TODO: add installation steps — Haleacas couldn't confidently \
                          detect one from the project's manifests._"
                    .to_string(),
            },
            auto_generated: install_command(fp).is_some(),
        },
        ReadmeSection {
            id: "usage".to_string(),
            kind: ReadmeSectionKind::Usage,
            title: "Usage".to_string(),
            content: "_TODO: add a usage example._".to_string(),
            auto_generated: false,
        },
        ReadmeSection {
            id: "license".to_string(),
            kind: ReadmeSectionKind::License,
            title: "License".to_string(),
            content: match &fp.license {
                Some(l) => format!("{l} — see [LICENSE](./LICENSE)."),
                None => "_TODO: add a LICENSE file and reference it here._".to_string(),
            },
            auto_generated: fp.license.is_some(),
        },
    ]
}

/// The full "Add Section" picker list (spec section 16's 16 named
/// sections — `Custom` isn't offered here since it's not something you
/// pick from a list of named things; the frontend's "add a custom
/// section" affordance sets that kind directly with a user-typed title).
pub fn catalog() -> Vec<ReadmeCatalogEntry> {
    use ReadmeSectionKind::*;
    [
        (Hero, "Hero"),
        (Description, "Description"),
        (Features, "Features"),
        (Screenshots, "Screenshots"),
        (Demo, "Demo"),
        (Installation, "Installation"),
        (QuickStart, "Quick Start"),
        (Usage, "Usage"),
        (Configuration, "Configuration"),
        (Architecture, "Architecture"),
        (Api, "API"),
        (Roadmap, "Roadmap"),
        (Contributing, "Contributing"),
        (License, "License"),
        (Support, "Support"),
        (Acknowledgements, "Acknowledgements"),
    ]
    .into_iter()
    .map(|(kind, default_title)| ReadmeCatalogEntry { kind, default_title: default_title.to_string() })
    .collect()
}

/// Assembles the ordered sections into one README.md. `Hero` renders as
/// the document's H1 (a title + tagline, not a subsection of itself);
/// everything else is an `##` heading, in whatever order the sections
/// list is in — reordering the list is the entire reorder feature, no
/// separate "order" field needed.
pub fn render_markdown(sections: &[ReadmeSection]) -> String {
    let mut out = String::new();
    for section in sections {
        if section.kind == ReadmeSectionKind::Hero {
            out.push_str(&format!("# {}\n\n{}\n\n", section.title, section.content));
        } else {
            out.push_str(&format!("## {}\n\n{}\n\n", section.title, section.content));
        }
    }
    let trimmed = out.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GitInfo;

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
            health: Default::default(),
            security_findings: vec![],
            security_findings_truncated: false,
        }
    }

    #[test]
    fn generate_seeds_four_sections_from_real_facts() {
        let mut fp = base_fp();
        fp.description = Some("A theme builder".to_string());
        fp.license = Some("MIT".to_string());
        fp.techs = vec![crate::types::TechDetection {
            name: "Rust".to_string(),
            evidence: "Cargo.toml present".to_string(),
        }];

        let sections = generate_sections(&fp);
        assert_eq!(sections.len(), 4);

        let hero = &sections[0];
        assert_eq!(hero.kind, ReadmeSectionKind::Hero);
        assert_eq!(hero.title, "demo");
        assert_eq!(hero.content, "A theme builder");
        assert!(hero.auto_generated);

        let install = &sections[1];
        assert!(install.content.contains("cargo build"));
        assert!(install.auto_generated);

        let usage = &sections[2];
        assert!(!usage.auto_generated);
        assert!(usage.content.contains("TODO"));

        let license = &sections[3];
        assert!(license.content.contains("MIT"));
        assert!(license.auto_generated);
    }

    #[test]
    fn generate_placeholders_rather_than_guessing_when_facts_are_missing() {
        let sections = generate_sections(&base_fp());
        assert!(sections[0].content.contains("TODO"));
        assert!(!sections[0].auto_generated);
        assert!(sections[1].content.contains("couldn't confidently detect"));
        assert!(!sections[1].auto_generated);
        assert!(sections[3].content.contains("TODO"));
        assert!(!sections[3].auto_generated);
    }

    #[test]
    fn catalog_has_sixteen_named_kinds_and_excludes_custom() {
        let entries = catalog();
        assert_eq!(entries.len(), 16);
        assert!(!entries.iter().any(|e| e.kind == ReadmeSectionKind::Custom));
        assert!(entries.iter().any(|e| e.kind == ReadmeSectionKind::Api && e.default_title == "API"));
    }

    #[test]
    fn render_puts_hero_as_h1_and_others_as_h2_in_list_order() {
        let sections = vec![
            ReadmeSection {
                id: "hero".to_string(),
                kind: ReadmeSectionKind::Hero,
                title: "demo".to_string(),
                content: "A theme builder".to_string(),
                auto_generated: true,
            },
            ReadmeSection {
                id: "license".to_string(),
                kind: ReadmeSectionKind::License,
                title: "License".to_string(),
                content: "MIT".to_string(),
                auto_generated: true,
            },
        ];
        let md = render_markdown(&sections);
        assert!(md.starts_with("# demo\n\nA theme builder\n\n## License\n\nMIT"));
    }

    #[test]
    fn render_respects_reordering() {
        let mut sections = generate_sections(&base_fp());
        sections.swap(1, 2); // Usage before Installation
        let md = render_markdown(&sections);
        let usage_pos = md.find("## Usage").unwrap();
        let install_pos = md.find("## Installation").unwrap();
        assert!(usage_pos < install_pos);
    }

    #[test]
    fn render_empty_sections_yields_empty_string() {
        assert_eq!(render_markdown(&[]), "");
    }
}
