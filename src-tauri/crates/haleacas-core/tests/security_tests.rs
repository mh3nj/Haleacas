mod common;

use common::write_file;
use haleacas_core::security::scan;
use tempfile::tempdir;

// All "keys" below are fixtures constructed only to match the regex shape —
// none are real credentials, and each is clearly fake-looking on inspection.

#[test]
fn flags_an_aws_style_access_key_id() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "config.py",
        "AWS_KEY = \"AKIAABCDEFGHIJKLMNOP\"\n",
    );

    let result = scan(tmp.path());

    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].file, "config.py");
    assert_eq!(result.findings[0].line, 1);
    assert!(result.findings[0].reason.contains("AWS access key"));
}

#[test]
fn finding_never_contains_the_actual_matched_text() {
    let tmp = tempdir().unwrap();
    let fake_key = "AKIAABCDEFGHIJKLMNOP";
    write_file(tmp.path(), "config.py", &format!("AWS_KEY = \"{fake_key}\"\n"));

    let result = scan(tmp.path());

    for finding in &result.findings {
        assert!(!finding.reason.contains(fake_key));
        assert!(!finding.file.contains(fake_key));
    }
}

#[test]
fn flags_a_private_key_block() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "id_rsa",
        "-----BEGIN RSA PRIVATE KEY-----\nMIIfakekeycontentfortest\n-----END RSA PRIVATE KEY-----\n",
    );

    let result = scan(tmp.path());

    assert!(result.findings.iter().any(|f| f.reason.contains("private key")));
}

#[test]
fn flags_a_github_style_token() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        ".env",
        "GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123456789\n",
    );

    let result = scan(tmp.path());

    assert!(result.findings.iter().any(|f| f.reason.contains("GitHub")));
}

#[test]
fn ordinary_source_code_produces_no_findings() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "src/main.rs",
        "fn main() {\n    println!(\"hello, world\");\n    let x = 42;\n}\n",
    );
    write_file(tmp.path(), "README.md", "# Demo\n\nJust a normal project.\n");

    let result = scan(tmp.path());

    assert!(result.findings.is_empty());
}

#[test]
fn skips_lockfiles_even_if_they_contain_matching_looking_strings() {
    let tmp = tempdir().unwrap();
    // A lockfile-shaped hash string that happens to be 40 chars, similar
    // in shape to what the AWS-secret pattern looks for, but in a file
    // that should never be scanned at all.
    write_file(
        tmp.path(),
        "package-lock.json",
        "{ \"integrity\": \"sha512-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\" }",
    );

    let result = scan(tmp.path());

    assert!(result.findings.is_empty());
}

#[test]
fn skips_files_in_noisy_directories() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "node_modules/some-dep/secret.js",
        "const key = \"AKIAABCDEFGHIJKLMNOP\";\n",
    );

    let result = scan(tmp.path());

    assert!(result.findings.is_empty());
}

#[test]
fn skips_binary_files() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("data.bin");
    // A null byte makes this invalid UTF-8, so read_to_string (and thus
    // the scanner) should skip it outright.
    std::fs::write(&path, [0u8, 159, 146, 150, 0u8, 1, 2, 3]).unwrap();

    let result = scan(tmp.path());

    assert!(result.findings.is_empty());
}

#[test]
fn reports_the_file_relative_to_the_repository_root() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "nested/dir/config.py",
        "AWS_KEY = \"AKIAABCDEFGHIJKLMNOP\"\n",
    );

    let result = scan(tmp.path());

    assert_eq!(result.findings[0].file, "nested/dir/config.py");
}

#[test]
fn generic_api_key_assignment_pattern_matches_quoted_long_values() {
    let tmp = tempdir().unwrap();
    write_file(
        tmp.path(),
        "settings.py",
        "SECRET_KEY = \"this-is-a-fake-django-secret-key-value\"\n",
    );

    let result = scan(tmp.path());

    assert!(!result.findings.is_empty());
}
