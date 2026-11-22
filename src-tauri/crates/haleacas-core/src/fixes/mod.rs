//! Orchestrates the safe, one-click fixes from spec section 11.
//!
//! Two hard rules from the spec, enforced here rather than trusted to
//! callers:
//! - "Every modification must be previewable before execution" —
//!   `preview()` computes exactly what `apply()` would write, with no
//!   disk access of its own.
//! - "Never silently overwrite existing files" — `apply()` re-checks the
//!   target file's existence itself right before writing, and refuses if
//!   it's there. A fix whose target already exists is also reported as
//!   `available: false` by `available_fixes()`, so the UI shouldn't even
//!   offer it — but `apply()` doesn't trust the UI to have checked.

pub mod gitignore;
pub mod licenses;
pub mod readme;

use crate::types::{FixKind, FixOption, FixParams, FixPreview, RepositoryFingerprint};
use chrono::Datelike;
use std::fs;
use std::path::{Path, PathBuf};

const ALL_KINDS: [FixKind; 3] = [
    FixKind::CreateGitignore,
    FixKind::CreateLicense,
    FixKind::CreateReadmeSkeleton,
];

fn target_file(kind: FixKind) -> &'static str {
    match kind {
        FixKind::CreateGitignore => ".gitignore",
        FixKind::CreateLicense => "LICENSE",
        FixKind::CreateReadmeSkeleton => "README.md",
    }
}

fn label(kind: FixKind) -> &'static str {
    match kind {
        FixKind::CreateGitignore => "Create .gitignore",
        FixKind::CreateLicense => "Create LICENSE",
        FixKind::CreateReadmeSkeleton => "Generate basic README",
    }
}

/// Which of the fixes already have their target file present, i.e. which
/// findings this fingerprint already resolved. Reused by both
/// `available_fixes` and `apply` so the "already exists" logic lives in
/// exactly one place.
fn already_present(fp: &RepositoryFingerprint, kind: FixKind) -> bool {
    match kind {
        FixKind::CreateGitignore => fp.has_gitignore,
        FixKind::CreateLicense => fp.license.is_some(),
        FixKind::CreateReadmeSkeleton => fp.has_readme,
    }
}

pub fn available_fixes(fp: &RepositoryFingerprint) -> Vec<FixOption> {
    ALL_KINDS
        .iter()
        .map(|&kind| {
            let present = already_present(fp, kind);
            FixOption {
                kind,
                label: label(kind).to_string(),
                target_file: target_file(kind).to_string(),
                available: !present,
                unavailable_reason: present
                    .then(|| format!("{} already exists.", target_file(kind))),
            }
        })
        .collect()
}

pub fn preview(
    fp: &RepositoryFingerprint,
    kind: FixKind,
    params: &FixParams,
) -> Result<FixPreview, String> {
    let content = match kind {
        FixKind::CreateGitignore => gitignore::generate(&fp.techs),
        FixKind::CreateLicense => {
            let license_id = params
                .license_id
                .as_deref()
                .ok_or("A license must be selected.")?;
            let holder = params
                .copyright_holder
                .as_deref()
                .filter(|h| !h.trim().is_empty())
                .ok_or("A copyright holder name is required.")?;
            let year = chrono::Utc::now().year();
            licenses::render(license_id, year, holder)?
        }
        FixKind::CreateReadmeSkeleton => readme::generate(fp),
    };

    Ok(FixPreview {
        kind,
        target_file: target_file(kind).to_string(),
        content,
    })
}

/// Write a previewed fix to disk. Refuses if the target file exists,
/// regardless of what the fingerprint's cached `has_*` flags say — those
/// could be stale if the file was created after the last scan.
pub fn apply(
    fp: &RepositoryFingerprint,
    kind: FixKind,
    params: &FixParams,
) -> Result<PathBuf, String> {
    let preview = preview(fp, kind, params)?;
    let target = Path::new(&fp.path).join(&preview.target_file);

    if target.exists() {
        return Err(format!(
            "{} already exists — refusing to overwrite it.",
            preview.target_file
        ));
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&target, &preview.content).map_err(|e| e.to_string())?;

    Ok(target)
}
