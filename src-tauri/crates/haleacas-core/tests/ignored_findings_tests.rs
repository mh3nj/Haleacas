use haleacas_core::db;
use haleacas_core::types::IgnoredFinding;
use rusqlite::Connection;

fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE ignored_findings (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_path TEXT NOT NULL,
            file      TEXT NOT NULL,
            line      INTEGER NOT NULL,
            reason    TEXT NOT NULL,
            UNIQUE(repo_path, file, line, reason)
        );
        "#,
    )
    .unwrap();
    conn
}

fn finding(file: &str, line: usize, reason: &str) -> IgnoredFinding {
    IgnoredFinding {
        file: file.to_string(),
        line,
        reason: reason.to_string(),
    }
}

#[test]
fn add_then_list_round_trips() {
    let conn = fresh_db();
    let f = finding("a.py", 3, "Possible AWS access key ID");

    db::ignored_findings::add(&conn, "/repos/demo", &f).unwrap();
    let listed = db::ignored_findings::list(&conn, "/repos/demo").unwrap();

    assert_eq!(listed, vec![f]);
}

#[test]
fn adding_the_same_finding_twice_does_not_duplicate() {
    let conn = fresh_db();
    let f = finding("a.py", 3, "Possible AWS access key ID");

    db::ignored_findings::add(&conn, "/repos/demo", &f).unwrap();
    db::ignored_findings::add(&conn, "/repos/demo", &f).unwrap();

    let listed = db::ignored_findings::list(&conn, "/repos/demo").unwrap();
    assert_eq!(listed.len(), 1);
}

#[test]
fn remove_deletes_only_the_matching_entry() {
    let conn = fresh_db();
    let a = finding("a.py", 3, "Possible AWS access key ID");
    let b = finding("b.py", 5, "Possible GitHub access token");
    db::ignored_findings::add(&conn, "/repos/demo", &a).unwrap();
    db::ignored_findings::add(&conn, "/repos/demo", &b).unwrap();

    db::ignored_findings::remove(&conn, "/repos/demo", &a).unwrap();

    let listed = db::ignored_findings::list(&conn, "/repos/demo").unwrap();
    assert_eq!(listed, vec![b]);
}

#[test]
fn ignore_lists_are_scoped_per_repository() {
    let conn = fresh_db();
    let f = finding("a.py", 3, "Possible AWS access key ID");
    db::ignored_findings::add(&conn, "/repos/alpha", &f).unwrap();

    let alpha_list = db::ignored_findings::list(&conn, "/repos/alpha").unwrap();
    let beta_list = db::ignored_findings::list(&conn, "/repos/beta").unwrap();

    assert_eq!(alpha_list.len(), 1);
    assert!(beta_list.is_empty());
}

#[test]
fn remove_all_for_repo_clears_everything_for_that_repo_only() {
    let conn = fresh_db();
    db::ignored_findings::add(&conn, "/repos/alpha", &finding("a.py", 1, "x")).unwrap();
    db::ignored_findings::add(&conn, "/repos/alpha", &finding("b.py", 2, "y")).unwrap();
    db::ignored_findings::add(&conn, "/repos/beta", &finding("c.py", 3, "z")).unwrap();

    db::ignored_findings::remove_all_for_repo(&conn, "/repos/alpha").unwrap();

    assert!(db::ignored_findings::list(&conn, "/repos/alpha").unwrap().is_empty());
    assert_eq!(db::ignored_findings::list(&conn, "/repos/beta").unwrap().len(), 1);
}

#[test]
fn remove_by_path_prefix_clears_the_root_and_nested_repos() {
    let conn = fresh_db();
    db::ignored_findings::add(&conn, "/repos/roots/alpha", &finding("a.py", 1, "x")).unwrap();
    db::ignored_findings::add(&conn, "/repos/roots", &finding("b.py", 2, "y")).unwrap();
    db::ignored_findings::add(&conn, "/repos/other/beta", &finding("c.py", 3, "z")).unwrap();

    db::ignored_findings::remove_by_path_prefix(&conn, "/repos/roots").unwrap();

    assert!(db::ignored_findings::list(&conn, "/repos/roots/alpha").unwrap().is_empty());
    assert!(db::ignored_findings::list(&conn, "/repos/roots").unwrap().is_empty());
    assert_eq!(db::ignored_findings::list(&conn, "/repos/other/beta").unwrap().len(), 1);
}
