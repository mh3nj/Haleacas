use crate::types::{CategoryScore, HealthCheck};

/// Spec section 17's README Quality Analyzer — deliberately deeper than
/// the coarse `has_readme`/`license.is_some()` booleans the repo-level
/// Health Engine's Documentation category checks (spec section 9): this
/// actually reads the README's text for whether an "Installation"
/// heading has real content under it, not just whether the heading
/// exists. `Clarity` (one of the spec's listed axes) isn't scored as a
/// fully separate number — it's folded into "does the section actually
/// have a body, not just an empty heading" on each of the checks below,
/// since a truly free-form clarity score would just be a guess dressed
/// up as a number, which is exactly what this project avoids elsewhere.
pub fn analyze(markdown: &str, has_license_file: bool) -> CategoryScore {
    let doc = parse(markdown);

    let checks = vec![
        HealthCheck {
            label: "Project description".to_string(),
            passed: doc.has_description(),
            detail: if doc.has_description() {
                "Descriptive text appears before the first heading, or directly \
                 under the title heading."
                    .to_string()
            } else {
                "No descriptive text was found before the first heading, and the \
                 first heading (usually the title) has no real text under it \
                 either."
                    .to_string()
            },
            weight: 2,
        },
        HealthCheck {
            label: "Installation instructions".to_string(),
            passed: doc.has_substantial_section(&["install"]),
            detail: if doc.has_substantial_section(&["install"]) {
                "An installation-related heading has real content under it.".to_string()
            } else {
                "No heading matching \"install\" was found with real content \
                 under it."
                    .to_string()
            },
            weight: 3,
        },
        HealthCheck {
            label: "Usage example".to_string(),
            passed: doc.has_substantial_section(&["usage", "getting started", "quick start", "quickstart"]),
            detail: if doc.has_substantial_section(&["usage", "getting started", "quick start", "quickstart"]) {
                "A usage-related heading has real content under it.".to_string()
            } else {
                "No heading matching \"usage\", \"getting started\", or \"quick \
                 start\" was found with real content under it."
                    .to_string()
            },
            weight: 3,
        },
        HealthCheck {
            label: "License section".to_string(),
            passed: has_license_file || doc.has_heading_matching(&["licen"]),
            detail: if has_license_file {
                "A LICENSE file was detected in the repository.".to_string()
            } else if doc.has_heading_matching(&["licen"]) {
                "A license-related heading was found in the README.".to_string()
            } else {
                "No LICENSE file and no license-related heading were found.".to_string()
            },
            weight: 2,
        },
        HealthCheck {
            label: "Screenshots or visuals".to_string(),
            passed: doc.has_image || doc.has_heading_matching(&["screenshot", "demo", "preview"]),
            detail: if doc.has_image {
                "At least one image is embedded in the README.".to_string()
            } else if doc.has_heading_matching(&["screenshot", "demo", "preview"]) {
                "A screenshot/demo-related heading was found.".to_string()
            } else {
                "No embedded images and no screenshot/demo-related heading were \
                 found."
                    .to_string()
            },
            weight: 1,
        },
        HealthCheck {
            label: "Contributing guidance".to_string(),
            passed: doc.has_heading_matching(&["contribut"]),
            detail: if doc.has_heading_matching(&["contribut"]) {
                "A contributing-related heading was found.".to_string()
            } else {
                "No contributing-related heading was found.".to_string()
            },
            weight: 1,
        },
        HealthCheck {
            label: "Structured with multiple sections".to_string(),
            passed: doc.headings.len() >= 3,
            detail: format!("{} heading(s) found.", doc.headings.len()),
            weight: 2,
        },
        HealthCheck {
            label: "Badges".to_string(),
            passed: doc.has_badge,
            detail: if doc.has_badge {
                "A badge (linked image near the top) was found.".to_string()
            } else {
                "No badge-style linked image was found.".to_string()
            },
            weight: 1,
        },
    ];

    crate::health::scoring::category_score("README", checks)
}

struct ParsedReadme {
    preamble: String,
    headings: Vec<(String, String)>, // (heading text, body text)
    has_image: bool,
    has_badge: bool,
}

impl ParsedReadme {
    /// A description counts whether it's true preamble (text before any
    /// heading exists at all) *or* sits directly under the very first
    /// heading — the overwhelmingly common `# ProjectName\n\nWhat it
    /// does.` shape, which is exactly what the Hero section this same
    /// crate's `readme_studio::generate_sections` produces.
    ///
    /// `_TODO_` placeholder bodies (emitted by `generate_sections` when a
    /// fact couldn't be detected) never count as a real description, even
    /// though they're longer than 10 chars — they're an explicit marker
    /// of missing content, not content itself.
    fn has_description(&self) -> bool {
        if !self.preamble.trim().is_empty() && !is_placeholder(&self.preamble) {
            return true;
        }
        self.headings
            .first()
            .map(|(_, body)| body.trim().len() >= 10 && !is_placeholder(body))
            .unwrap_or(false)
    }

    fn has_heading_matching(&self, needles: &[&str]) -> bool {
        self.headings
            .iter()
            .any(|(h, _)| needles.iter().any(|n| h.to_lowercase().contains(n)))
    }

    /// A heading matches *and* has a non-trivial body under it — the
    /// "clarity" half of these checks, not just "the word appears".
    /// Placeholder `_TODO_` bodies don't count (see `has_description`).
    fn has_substantial_section(&self, needles: &[&str]) -> bool {
        self.headings.iter().any(|(h, body)| {
            needles.iter().any(|n| h.to_lowercase().contains(n))
                && body.trim().len() >= 10
                && !is_placeholder(body)
        })
    }
}

/// True when a section body is one of the `_TODO: ...` placeholders
/// `generate_sections` emits for facts it couldn't detect, rather than
/// real author-written content. Case-insensitive on purpose — the
/// placeholder convention is uppercase, but a lowercase `todo` note is
/// equally not finished documentation.
fn is_placeholder(body: &str) -> bool {
    body.to_lowercase().contains("todo")
}

fn parse(markdown: &str) -> ParsedReadme {
    let mut preamble = String::new();
    let mut headings: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            if let Some((h, b)) = current.take() {
                headings.push((h, b));
            }
            current = Some((trimmed.trim_start_matches('#').trim().to_string(), String::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        } else {
            preamble.push_str(line);
            preamble.push('\n');
        }
    }
    if let Some((h, b)) = current.take() {
        headings.push((h, b));
    }

    let has_image = markdown.contains("![");
    // A badge is the standard `[![alt](img-url)](link-url)` idiom — a
    // linked image — or a shields.io URL, which covers badges that
    // aren't wrapped in an outer link.
    let has_badge = markdown.contains("[![") || markdown.contains("shields.io");

    ParsedReadme { preamble, headings, has_image, has_badge }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_readme_scores_low_with_every_check_explained() {
        let report = analyze("", false);
        assert!(report.score < 20);
        assert!(report.checks.iter().all(|c| !c.detail.is_empty()));
    }

    #[test]
    fn heading_with_no_body_does_not_count_as_installation() {
        let md = "# demo\n\n## Installation\n\n## Usage\n\nRun it.\n";
        let report = analyze(md, false);
        let install = report.checks.iter().find(|c| c.label == "Installation instructions").unwrap();
        assert!(!install.passed);
    }

    #[test]
    fn heading_with_real_body_counts_as_installation() {
        let md = "# demo\n\n## Installation\n\nRun `npm install` to get started.\n\n## Usage\n\nThen run `npm start`.\n";
        let report = analyze(md, false);
        let install = report.checks.iter().find(|c| c.label == "Installation instructions").unwrap();
        assert!(install.passed);
        let usage = report.checks.iter().find(|c| c.label == "Usage example").unwrap();
        assert!(usage.passed);
    }

    #[test]
    fn license_file_counts_even_without_a_license_heading() {
        let md = "# demo\n\nA project.\n";
        let report = analyze(md, true);
        let license = report.checks.iter().find(|c| c.label == "License section").unwrap();
        assert!(license.passed);
    }

    #[test]
    fn detects_badges_and_images() {
        let md = "# demo\n\n[![Build](https://img.shields.io/badge/x)](https://ci.example.com)\n\n![Screenshot](./shot.png)\n";
        let report = analyze(md, false);
        let badges = report.checks.iter().find(|c| c.label == "Badges").unwrap();
        assert!(badges.passed);
        let visuals = report.checks.iter().find(|c| c.label == "Screenshots or visuals").unwrap();
        assert!(visuals.passed);
    }

    #[test]
    fn description_directly_under_title_heading_counts() {
        let md = "# demo\n\nA theme builder for Telegram, built with Tauri.\n\n## Usage\n\nRun it.\n";
        let report = analyze(md, false);
        let desc = report.checks.iter().find(|c| c.label == "Project description").unwrap();
        assert!(desc.passed);
    }

    #[test]
    fn well_formed_readme_scores_high() {
        let md = "# demo\n\nA theme builder for Telegram.\n\n\
                   [![Build](https://img.shields.io/badge/build-passing-green)](https://ci.example.com)\n\n\
                   ## Installation\n\nRun `npm install`.\n\n\
                   ## Usage\n\nRun `npm start` and open the app.\n\n\
                   ## Screenshots\n\n![app](./shot.png)\n\n\
                   ## Contributing\n\nPRs welcome, see CONTRIBUTING.md.\n\n\
                   ## License\n\nMIT.\n";
        let report = analyze(md, true);
        assert!(report.score >= 90, "expected a high score, got {}", report.score);
    }
}
