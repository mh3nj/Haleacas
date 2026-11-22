use crate::types::RepositoryFingerprint;
use rusqlite::{params, Connection, OptionalExtension};

/// Persist (insert or replace) a computed fingerprint, keyed by its stable
/// path-derived id.
pub fn upsert(conn: &Connection, fp: &RepositoryFingerprint) -> anyhow::Result<()> {
    let json = serde_json::to_string(fp)?;
    conn.execute(
        "INSERT INTO repositories (id, path, name, fingerprint, last_scanned)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            path = excluded.path,
            name = excluded.name,
            fingerprint = excluded.fingerprint,
            last_scanned = excluded.last_scanned",
        params![fp.id, fp.path, fp.name, json, fp.scanned_at],
    )?;
    Ok(())
}

/// Load every cached fingerprint, most recently scanned first.
pub fn list(conn: &Connection) -> anyhow::Result<Vec<RepositoryFingerprint>> {
    let mut stmt =
        conn.prepare("SELECT fingerprint FROM repositories ORDER BY last_scanned DESC")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        let json = row?;
        out.push(serde_json::from_str(&json)?);
    }
    Ok(out)
}

/// Load a single cached fingerprint by its unique path, if present. Used
/// by incremental scanning to check "have we seen this repo before, and
/// what did it look like last time" before deciding whether a full
/// rescan is needed.
pub fn get_by_path(conn: &Connection, path: &str) -> anyhow::Result<Option<RepositoryFingerprint>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT fingerprint FROM repositories WHERE path = ?1",
            params![path],
            |r| r.get(0),
        )
        .optional()?;
    match raw {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

/// Load a single cached fingerprint by its stable id, if present.
pub fn get_by_id(conn: &Connection, id: &str) -> anyhow::Result<Option<RepositoryFingerprint>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT fingerprint FROM repositories WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()?;
    match raw {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

/// Remove a cached repository, e.g. after its root is removed from settings.
pub fn remove_by_path_prefix(conn: &Connection, prefix: &str) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM repositories WHERE path = ?1 OR path LIKE ?2",
        params![prefix, format!("{prefix}/%")],
    )?;
    Ok(())
}
