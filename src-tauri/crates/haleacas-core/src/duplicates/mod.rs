//! Local duplicate and similar-code detection (spec section 23).
//!
//! Two independent checks, both purely local and syntactic:
//!
//! 1. **Identical files** — an exact content hash match. Unambiguous,
//!    cheap, and reported with full confidence.
//! 2. **Highly similar files** — a Jaccard index (line-set overlap)
//!    between files of the *same extension*. This is a syntactic
//!    similarity score, not a semantic one — two files can score high
//!    here while doing completely different things if they happen to
//!    share a lot of boilerplate lines, and the spec is explicit that
//!    Haleacas shouldn't claim otherwise. `SimilarPair.similarity` is
//!    presented as exactly what it is: a line-overlap percentage.
//!
//! Both checks are bounded-cost heuristics (spec section 23: "start with
//! practical heuristics"), not exhaustive scans — see the caps below and
//! `DuplicateReport.truncated`.

use crate::skip_dirs::is_skipped_dir_name;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::types::{DuplicateGroup, DuplicateReport, SimilarPair};

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "rb", "java", "kt", "swift", "dart", "c", "h",
    "cpp", "hpp", "cc", "cs", "php", "vue", "svelte", "css", "scss", "html",
];

/// Skip files smaller than this — tiny stub files (a one-line index.ts
/// re-export, an empty `__init__.py`) trivially look "similar" to
/// everything and are just noise, not a meaningful finding.
const MIN_FILE_SIZE_BYTES: u64 = 50;
/// Skip files larger than this to bound read/hash cost.
const MAX_FILE_SIZE_BYTES: u64 = 512 * 1024;
/// Below this many non-blank lines, a Jaccard score isn't meaningful
/// (a 3-line file matching another 3-line file 100% is unsurprising).
const MIN_LINES_FOR_SIMILARITY: usize = 5;
const SIMILARITY_THRESHOLD: f64 = 0.75;
/// Per-extension comparison cap: within one extension group, only the
/// largest N files (by size) are compared pairwise against each other.
const CAP_PER_EXTENSION: usize = 100;
/// Hard ceiling on total pairwise comparisons across the whole scan, so a
/// pathological repo shape (many huge same-extension groups) can't make
/// a scan run unboundedly long.
const MAX_TOTAL_COMPARISONS: usize = 50_000;

struct Candidate {
    relative_path: String,
    extension: String,
    size: u64,
    hash: [u8; 32],
    lines: HashSet<String>,
    line_count: usize,
}

pub fn scan(root: &Path) -> DuplicateReport {
    let candidates = collect_candidates(root);
    let files_analyzed = candidates.len();

    let identical_groups = find_identical_groups(&candidates);

    let (similar_pairs, truncated_by_similarity) = find_similar_pairs(&candidates);

    DuplicateReport {
        identical_groups,
        similar_pairs,
        files_analyzed,
        truncated: truncated_by_similarity,
    }
}

fn collect_candidates(root: &Path) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        if !entry.file_type().is_dir() {
            return true;
        }
        !is_skipped_dir_name(&entry.file_name().to_string_lossy())
    });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !SOURCE_EXTENSIONS.contains(&ext) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.len() < MIN_FILE_SIZE_BYTES || meta.len() > MAX_FILE_SIZE_BYTES {
            continue;
        }
        let Ok(contents) = fs::read_to_string(entry.path()) else {
            continue;
        };

        let mut hasher = Sha256::new();
        hasher.update(contents.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();

        let lines: HashSet<String> = contents
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        let line_count = lines.len();

        let relative_path = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        candidates.push(Candidate {
            relative_path,
            extension: ext.to_string(),
            size: meta.len(),
            hash,
            lines,
            line_count,
        });
    }

    candidates
}

fn find_identical_groups(candidates: &[Candidate]) -> Vec<DuplicateGroup> {
    let mut by_hash: HashMap<[u8; 32], Vec<String>> = HashMap::new();
    for c in candidates {
        by_hash.entry(c.hash).or_default().push(c.relative_path.clone());
    }

    let mut groups: Vec<DuplicateGroup> = by_hash
        .into_values()
        .filter(|files| files.len() >= 2)
        .map(|mut files| {
            files.sort();
            DuplicateGroup { files }
        })
        .collect();

    groups.sort_by(|a, b| a.files[0].cmp(&b.files[0]));
    groups
}

fn find_similar_pairs(candidates: &[Candidate]) -> (Vec<SimilarPair>, bool) {
    let mut by_extension: HashMap<&str, Vec<&Candidate>> = HashMap::new();
    for c in candidates {
        by_extension.entry(c.extension.as_str()).or_default().push(c);
    }

    let mut pairs = Vec::new();
    let mut truncated = false;
    let mut comparisons = 0usize;

    'extensions: for group in by_extension.values_mut() {
        if group.len() > CAP_PER_EXTENSION {
            truncated = true;
            group.sort_by(|a, b| b.size.cmp(&a.size));
            group.truncate(CAP_PER_EXTENSION);
        }

        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                if comparisons >= MAX_TOTAL_COMPARISONS {
                    truncated = true;
                    break 'extensions;
                }
                comparisons += 1;

                let a = group[i];
                let b = group[j];
                if a.hash == b.hash {
                    continue; // already reported as an identical group
                }
                if a.line_count < MIN_LINES_FOR_SIMILARITY || b.line_count < MIN_LINES_FOR_SIMILARITY {
                    continue;
                }

                let similarity = jaccard(&a.lines, &b.lines);
                if similarity >= SIMILARITY_THRESHOLD {
                    let (file_a, file_b) = if a.relative_path <= b.relative_path {
                        (a.relative_path.clone(), b.relative_path.clone())
                    } else {
                        (b.relative_path.clone(), a.relative_path.clone())
                    };
                    pairs.push(SimilarPair {
                        file_a,
                        file_b,
                        similarity: (similarity * 100.0).round() as u8,
                    });
                }
            }
        }
    }

    pairs.sort_by(|a, b| b.similarity.cmp(&a.similarity).then(a.file_a.cmp(&b.file_a)));
    (pairs, truncated)
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}
