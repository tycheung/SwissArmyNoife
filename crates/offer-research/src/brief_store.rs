//! Harness-agnostic research brief persistence (`sak242`).

use rusqlite::Connection;
use types::ErrorCode;
use uuid::Uuid;

/// Stored brief artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
    pub id: String,
    pub title: String,
    pub body: String,
    pub source_url: Option<String>,
}

/// Upsert a brief; generates id when `id` is `None`.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on `SQLite` failure.
pub fn put_brief(
    conn: &Connection,
    id: Option<&str>,
    title: &str,
    body: &str,
    source_url: Option<&str>,
) -> Result<Brief, ErrorCode> {
    let id = id
        .filter(|s| !s.trim().is_empty())
        .map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
    conn.execute(
        "INSERT INTO research_briefs (id, title, body, source_url, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
           title=excluded.title,
           body=excluded.body,
           source_url=excluded.source_url",
        rusqlite::params![id, title, body, source_url],
    )
    .map_err(|_| ErrorCode::SchemaInvalid)?;
    Ok(Brief {
        id,
        title: title.to_owned(),
        body: body.to_owned(),
        source_url: source_url.map(str::to_owned),
    })
}

/// Load a brief by id.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on `SQLite` failure.
pub fn get_brief(conn: &Connection, id: &str) -> Result<Option<Brief>, ErrorCode> {
    let mut stmt = conn
        .prepare("SELECT id, title, body, source_url FROM research_briefs WHERE id = ?1")
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    let mut rows = stmt
        .query(rusqlite::params![id])
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    match rows.next().map_err(|_| ErrorCode::SchemaInvalid)? {
        Some(row) => Ok(Some(Brief {
            id: row.get(0).map_err(|_| ErrorCode::SchemaInvalid)?,
            title: row.get(1).map_err(|_| ErrorCode::SchemaInvalid)?,
            body: row.get(2).map_err(|_| ErrorCode::SchemaInvalid)?,
            source_url: row.get(3).map_err(|_| ErrorCode::SchemaInvalid)?,
        })),
        None => Ok(None),
    }
}

/// List briefs (newest first), capped.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on `SQLite` failure.
pub fn list_briefs(conn: &Connection, limit: usize) -> Result<Vec<Brief>, ErrorCode> {
    let limit = i64::try_from(limit.clamp(1, 100)).unwrap_or(20);
    let mut stmt = conn
        .prepare(
            "SELECT id, title, body, source_url FROM research_briefs
             ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    let rows = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(Brief {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                source_url: row.get(3)?,
            })
        })
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|_| ErrorCode::SchemaInvalid)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use persist_sqlite::open_and_migrate;

    #[test]
    fn put_get_list() {
        let tmp = tempfile::tempdir().expect("tmp");
        let conn = open_and_migrate(&tmp.path().join("b.db")).expect("db");
        let b = put_brief(&conn, None, "t", "body", Some("https://x")).expect("put");
        let got = get_brief(&conn, &b.id).expect("get").expect("some");
        assert_eq!(got.title, "t");
        let listed = list_briefs(&conn, 10).expect("list");
        assert_eq!(listed.len(), 1);
    }
}
