mod common;

use common::write_file;
use haleacas_core::duplicates::scan;
use tempfile::tempdir;

fn source_block(seed: &str) -> String {
    // A reasonably long, distinctive block of "source" so it clears the
    // minimum-size and minimum-line thresholds.
    (0..12)
        .map(|i| format!("line_{i}_{seed}();"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn detects_two_byte_identical_files() {
    let tmp = tempdir().unwrap();
    let content = source_block("a");
    write_file(tmp.path(), "src/one.rs", &content);
    write_file(tmp.path(), "src/two.rs", &content);

    let report = scan(tmp.path());

    assert_eq!(report.identical_groups.len(), 1);
    assert_eq!(report.identical_groups[0].files, vec!["src/one.rs", "src/two.rs"]);
}

#[test]
fn groups_three_identical_files_together() {
    let tmp = tempdir().unwrap();
    let content = source_block("a");
    write_file(tmp.path(), "a.rs", &content);
    write_file(tmp.path(), "b.rs", &content);
    write_file(tmp.path(), "c.rs", &content);

    let report = scan(tmp.path());

    assert_eq!(report.identical_groups.len(), 1);
    assert_eq!(report.identical_groups[0].files.len(), 3);
}

#[test]
fn distinct_files_produce_no_identical_group() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "a.rs", &source_block("a"));
    write_file(tmp.path(), "b.rs", &source_block("b"));

    let report = scan(tmp.path());

    assert!(report.identical_groups.is_empty());
}

#[test]
fn near_identical_files_are_reported_as_similar_not_identical() {
    let tmp = tempdir().unwrap();
    let base = source_block("a");
    // One line different out of 12 -> high but not perfect overlap.
    let modified = base.replace("line_0_a", "line_0_CHANGED");

    write_file(tmp.path(), "a.rs", &base);
    write_file(tmp.path(), "b.rs", &modified);

    let report = scan(tmp.path());

    assert!(report.identical_groups.is_empty());
    assert_eq!(report.similar_pairs.len(), 1);
    assert_eq!(report.similar_pairs[0].file_a, "a.rs");
    assert_eq!(report.similar_pairs[0].file_b, "b.rs");
    assert!(report.similar_pairs[0].similarity >= 75);
    assert!(report.similar_pairs[0].similarity < 100);
}

#[test]
fn completely_different_files_are_not_flagged_as_similar() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "a.rs", &source_block("alpha"));
    write_file(
        tmp.path(),
        "b.rs",
        "totally\nunrelated\ncontent\nwith\ncompletely\ndifferent\nwords\nhere\nfor\nsure\nyes\nreally",
    );

    let report = scan(tmp.path());

    assert!(report.similar_pairs.is_empty());
}

#[test]
fn different_extensions_are_never_compared_to_each_other() {
    let tmp = tempdir().unwrap();
    let content = source_block("a");
    write_file(tmp.path(), "a.rs", &content);
    write_file(tmp.path(), "a.py", &content); // identical content, different ext

    let report = scan(tmp.path());

    // Same content but different extensions: not grouped as identical
    // (identical grouping is purely by content hash, so this WOULD match)
    // — but must never appear in similar_pairs, which is extension-scoped.
    assert!(report.similar_pairs.is_empty());
}

#[test]
fn tiny_files_are_excluded_to_avoid_noise() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "a.rs", "x");
    write_file(tmp.path(), "b.rs", "x");

    let report = scan(tmp.path());

    assert!(report.identical_groups.is_empty());
    assert_eq!(report.files_analyzed, 0);
}

#[test]
fn non_source_extensions_are_ignored() {
    let tmp = tempdir().unwrap();
    let content = source_block("a");
    write_file(tmp.path(), "notes.txt", &content);
    write_file(tmp.path(), "notes-copy.txt", &content);

    let report = scan(tmp.path());

    assert_eq!(report.files_analyzed, 0);
}

#[test]
fn skips_noisy_directories() {
    let tmp = tempdir().unwrap();
    let content = source_block("a");
    write_file(tmp.path(), "node_modules/dep/index.js", &content);
    write_file(tmp.path(), "node_modules/dep/copy.js", &content);

    let report = scan(tmp.path());

    assert!(report.identical_groups.is_empty());
}

#[test]
fn files_analyzed_counts_candidates_that_passed_filters() {
    let tmp = tempdir().unwrap();
    write_file(tmp.path(), "a.rs", &source_block("a"));
    write_file(tmp.path(), "b.rs", &source_block("b"));
    write_file(tmp.path(), "tiny.rs", "x"); // filtered out (too small)

    let report = scan(tmp.path());

    assert_eq!(report.files_analyzed, 2);
}

#[test]
fn per_extension_cap_marks_the_report_truncated() {
    let tmp = tempdir().unwrap();
    // One more file than CAP_PER_EXTENSION (100) in the same extension
    // group, each with distinct content so none of them would otherwise
    // match — this isolates the cap behavior from the similarity logic.
    for i in 0..101 {
        write_file(tmp.path(), &format!("file{i}.rs"), &source_block(&format!("unique{i}")));
    }

    let report = scan(tmp.path());

    assert!(report.truncated, "expected the 101-file group to trip the per-extension cap");
    // files_analyzed still reflects everything that passed the initial
    // size/extension filter, even though not all of them were compared.
    assert_eq!(report.files_analyzed, 101);
}
