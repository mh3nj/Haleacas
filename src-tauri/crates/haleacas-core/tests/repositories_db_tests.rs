use haleacas_core::db;
use haleacas_core::types::{GitInfo, HealthReport, RepositoryFingerprint};
use rusqlite::Connection;

fn fresh_db() -> Connection {
    // An in-memory connection with the same schema `db::open()` would
    // apply to a real file, without touching disk or the OS data dir.
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE repositories (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            fingerprint TEXT NOT NULL,
            last_scanned TEXT NOT NULL
        );
        "#,
    )
    .unwrap();
    conn
}

fn sample(id: &str, name: &str, scanned_at: &str) -> RepositoryFingerprint {
    RepositoryFingerprint {
        id: id.to_string(),
        name: name.to_string(),
        path: format!("/repos/{name}"),
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
        scanned_at: scanned_at.to_string(),
        health: HealthReport::default(),
        security_findings: vec![],
        security_findings_truncated: false,
    }
}

#[test]
fn upsert_then_get_by_id_round_trips() {
    let conn = fresh_db();
    let fp = sample("abc123", "demo", "2026-01-01T00:00:00Z");

    db::repositories::upsert(&conn, &fp).unwrap();
    let loaded = db::repositories::get_by_id(&conn, "abc123").unwrap();

    assert_eq!(loaded.unwrap().name, "demo");
}

#[test]
fn get_by_id_returns_none_for_unknown_id() {
    let conn = fresh_db();

    let loaded = db::repositories::get_by_id(&conn, "does-not-exist").unwrap();

    assert!(loaded.is_none());
}

#[test]
fn get_by_path_finds_a_stored_fingerprint() {
    let conn = fresh_db();
    let mut fp = sample("abc123", "demo", "2026-01-01T00:00:00Z");
    fp.path = "/repos/demo".to_string();
    db::repositories::upsert(&conn, &fp).unwrap();

    let loaded = db::repositories::get_by_path(&conn, "/repos/demo").unwrap();

    assert_eq!(loaded.unwrap().id, "abc123");
}

#[test]
fn get_by_path_returns_none_for_unknown_path() {
    let conn = fresh_db();

    let loaded = db::repositories::get_by_path(&conn, "/nowhere").unwrap();

    assert!(loaded.is_none());
}

#[test]
fn upsert_on_existing_id_replaces_rather_than_duplicates() {
    let conn = fresh_db();
    let first = sample("abc123", "demo", "2026-01-01T00:00:00Z");
    let updated = sample("abc123", "demo-renamed", "2026-01-02T00:00:00Z");

    db::repositories::upsert(&conn, &first).unwrap();
    db::repositories::upsert(&conn, &updated).unwrap();

    let all = db::repositories::list(&conn).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "demo-renamed");
}

#[test]
fn list_orders_most_recently_scanned_first() {
    let conn = fresh_db();
    db::repositories::upsert(&conn, &sample("a", "alpha", "2026-01-01T00:00:00Z")).unwrap();
    db::repositories::upsert(&conn, &sample("b", "beta", "2026-01-03T00:00:00Z")).unwrap();
    db::repositories::upsert(&conn, &sample("c", "gamma", "2026-01-02T00:00:00Z")).unwrap();

    let all = db::repositories::list(&conn).unwrap();

    assert_eq!(
        all.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        vec!["beta", "gamma", "alpha"]
    );
}

#[test]
fn remove_by_path_prefix_deletes_the_root_and_its_children() {
    let conn = fresh_db();
    let mut inside_root = sample("a", "alpha", "2026-01-01T00:00:00Z");
    inside_root.path = "/repos/roots/alpha".to_string();
    let mut also_root_itself = sample("b", "roots", "2026-01-01T00:00:00Z");
    also_root_itself.path = "/repos/roots".to_string();
    let mut outside_root = sample("c", "beta", "2026-01-01T00:00:00Z");
    outside_root.path = "/repos/other/beta".to_string();

    db::repositories::upsert(&conn, &inside_root).unwrap();
    db::repositories::upsert(&conn, &also_root_itself).unwrap();
    db::repositories::upsert(&conn, &outside_root).unwrap();

    db::repositories::remove_by_path_prefix(&conn, "/repos/roots").unwrap();

    let remaining = db::repositories::list(&conn).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name, "beta");
}
