mod common;

use common::write_file;
use haleacas_core::scanner::fingerprint::build;
use haleacas_core::search::search;
use haleacas_core::types::SearchOptions;
use tempfile::tempdir;

#[test]
fn finds_a_match_in_one_of_two_repositories() {
    let tmp = tempdir().unwrap();
    let repo_a = tmp.path().join("alpha");
    let repo_b = tmp.path().join("beta");
    std::fs::create_dir_all(&repo_a).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    write_file(&repo_a, "src/auth.ts", "function login() {}\n// TODO: add 2FA\n");
    write_file(&repo_b, "src/main.rs", "fn main() {}\n");

    let fps = vec![build(&repo_a), build(&repo_b)];
    let report = search(&fps, "TODO", &SearchOptions::default());

    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.matches[0].repo_name, "alpha");
    assert_eq!(report.matches[0].file, "src/auth.ts");
    assert_eq!(report.matches[0].line, 2);
}

#[test]
fn default_search_is_case_insensitive() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "notes.txt", "a fixme here\n");

    let fps = vec![build(&repo)];
    let report = search(&fps, "FIXME", &SearchOptions::default());

    assert_eq!(report.matches.len(), 1);
}

#[test]
fn case_sensitive_option_distinguishes_case() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "config.py", "api_key = 'x'\nAPI_KEY = 'y'\n");

    let fps = vec![build(&repo)];
    let options = SearchOptions {
        case_sensitive: true,
        ..Default::default()
    };
    let report = search(&fps, "API_KEY", &options);

    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.matches[0].line, 2);
}

#[test]
fn extension_filter_restricts_matches() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "a.ts", "localhost:3000\n");
    write_file(&repo, "a.md", "localhost:3000\n");

    let fps = vec![build(&repo)];
    let options = SearchOptions {
        extension: Some("ts".to_string()),
        ..Default::default()
    };
    let report = search(&fps, "localhost", &options);

    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.matches[0].file, "a.ts");
}

#[test]
fn repo_id_filter_restricts_to_one_repository() {
    let tmp = tempdir().unwrap();
    let repo_a = tmp.path().join("alpha");
    let repo_b = tmp.path().join("beta");
    std::fs::create_dir_all(&repo_a).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    write_file(&repo_a, "a.txt", "needle\n");
    write_file(&repo_b, "b.txt", "needle\n");

    let fp_a = build(&repo_a);
    let fp_b = build(&repo_b);
    let options = SearchOptions {
        repo_id: Some(fp_a.id.clone()),
        ..Default::default()
    };
    let report = search(&[fp_a, fp_b], "needle", &options);

    assert_eq!(report.matches.len(), 1);
    assert_eq!(report.repositories_searched, 1);
}

#[test]
fn empty_query_returns_no_matches_without_scanning() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "a.txt", "anything\n");

    let fps = vec![build(&repo)];
    let report = search(&fps, "   ", &SearchOptions::default());

    assert!(report.matches.is_empty());
    assert_eq!(report.repositories_searched, 0);
}

#[test]
fn skips_lockfiles() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "package-lock.json", "needle needle needle\n");

    let fps = vec![build(&repo)];
    let report = search(&fps, "needle", &SearchOptions::default());

    assert!(report.matches.is_empty());
}

#[test]
fn skips_binary_files() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("data.bin"), [0u8, 1, 2, 3, 0u8]).unwrap();

    let fps = vec![build(&repo)];
    let report = search(&fps, "anything", &SearchOptions::default());

    assert!(report.matches.is_empty());
}

#[test]
fn per_file_match_cap_is_reported_as_truncated() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let content = "needle\n".repeat(25);
    write_file(&repo, "many.txt", &content);

    let fps = vec![build(&repo)];
    let report = search(&fps, "needle", &SearchOptions::default());

    assert_eq!(report.matches.len(), 20);
    assert!(report.truncated);
}

#[test]
fn line_text_is_captured_and_trimmed() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "a.txt", "   needle with padding   \n");

    let fps = vec![build(&repo)];
    let report = search(&fps, "needle", &SearchOptions::default());

    assert_eq!(report.matches[0].line_text, "needle with padding");
}

#[test]
fn path_substring_filter_restricts_to_matching_paths() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    write_file(&repo, "src/auth.ts", "needle here\n");
    write_file(&repo, "docs/notes.md", "needle here too\n");

    let fps = vec![build(&repo)];
    let options = SearchOptions {
        path_substring: Some("src".to_string()),
        ..Default::default()
    };
    let report = search(&fps, "needle", &options);

    assert_eq!(report.matches.len(), 1);
    assert!(report.matches[0].file.contains("src"));
}
