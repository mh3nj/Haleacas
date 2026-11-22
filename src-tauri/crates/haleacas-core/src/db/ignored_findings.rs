use crate::types::IgnoredFinding;
use rusqlite::{params, Connection};

/// Mark a finding as ignored for a given repository. Idempotent — adding
/// the same (file, line, reason) twice is a no-op, not a duplicate row.
pub fn add(conn: &Connection, repo_path: &str, finding: &IgnoredFinding) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO ignored_findings (repo_path, file, line, reason)
         VALUES (?1, ?2, ?3, ?4)",
        params![repo_path, finding.file, finding.line, finding.reason],
    )?;
    Ok(())
}

/// Un-ignore a finding — it will show up again on the next scan.
pub fn remove(conn: &Connection, repo_path: &str, finding: &IgnoredFinding) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM ignored_findings
         WHERE repo_path = ?1 AND file = ?2 AND line = ?3 AND reason = ?4",
        params![repo_path, finding.file, finding.line, finding.reason],
    )?;
    Ok(())
}

pub fn list(conn: &Connection, repo_path: &str) -> anyhow::Result<Vec<IgnoredFinding>> {
    let mut stmt = conn.prepare(
        "SELECT file, line, reason FROM ignored_findings WHERE repo_path = ?1 ORDER BY file, line",
    )?;
    let rows = stmt.query_map(params![repo_path], |r| {
        Ok(IgnoredFinding {
            file: r.get(0)?,
            line: r.get(1)?,
            reason: r.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Remove every ignored finding for a repository, e.g. when its root is
/// removed from settings — mirrors `repositories::remove_by_path_prefix`.
pub fn remove_all_for_repo(conn: &Connection, repo_path: &str) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM ignored_findings WHERE repo_path = ?1",
        params![repo_path],
    )?;
    Ok(())
}

/// Remove every ignored finding under a removed root, e.g. when a
/// configured root (possibly containing several repositories) is
/// removed from settings. Mirrors
/// `repositories::remove_by_path_prefix`'s prefix semantics.
pub fn remove_by_path_prefix(conn: &Connection, prefix: &str) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM ignored_findings WHERE repo_path = ?1 OR repo_path LIKE ?2",
        params![prefix, format!("{prefix}/%")],
    )?;
    Ok(())
}
