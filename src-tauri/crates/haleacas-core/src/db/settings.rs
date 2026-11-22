use crate::types::Settings;
use rusqlite::{params, Connection, OptionalExtension};

const KEY: &str = "settings";

pub fn load(conn: &Connection) -> anyhow::Result<Settings> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![KEY],
            |r| r.get(0),
        )
        .optional()?;

    match raw {
        Some(json) => Ok(serde_json::from_str(&json)?),
        None => {
            let defaults = Settings::default();
            save(conn, &defaults)?;
            Ok(defaults)
        }
    }
}

pub fn save(conn: &Connection, settings: &Settings) -> anyhow::Result<()> {
    let json = serde_json::to_string(settings)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![KEY, json],
    )?;
    Ok(())
}
