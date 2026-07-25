//! Persist catalog offers (`catalog_offers` table).

use rusqlite::Connection;

use crate::Result;

/// One row from `catalog_offers`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogOfferRow {
    pub offer_id: String,
    pub version: String,
    pub origin: String,
}

/// Insert or replace a catalog offer.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn upsert_offer(conn: &Connection, offer_id: &str, version: &str, origin: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO catalog_offers (offer_id, version, origin)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(offer_id) DO UPDATE SET
           version = excluded.version,
           origin = excluded.origin",
        [offer_id, version, origin],
    )?;
    Ok(())
}

/// Fetch one offer by id.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn get_offer(conn: &Connection, offer_id: &str) -> Result<Option<CatalogOfferRow>> {
    let mut stmt =
        conn.prepare("SELECT offer_id, version, origin FROM catalog_offers WHERE offer_id = ?1")?;
    let mut rows = stmt.query([offer_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(CatalogOfferRow {
        offer_id: row.get(0)?,
        version: row.get(1)?,
        origin: row.get(2)?,
    }))
}

/// List all offers ordered by `offer_id`.
///
/// # Errors
/// Returns [`crate::PersistError::Sqlite`] on database failure.
pub fn list_offers(conn: &Connection) -> Result<Vec<CatalogOfferRow>> {
    let mut stmt =
        conn.prepare("SELECT offer_id, version, origin FROM catalog_offers ORDER BY offer_id ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(CatalogOfferRow {
            offer_id: row.get(0)?,
            version: row.get(1)?,
            origin: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{open_and_migrate, CONFIG_DIR, DB_PATH};

    fn with_conn(test: impl FnOnce(&Connection)) {
        let _guard = crate::ENV_LOCK.lock().expect("env lock");
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var(CONFIG_DIR, tmp.path());
        std::env::remove_var(DB_PATH);
        let path = tmp.path().join("broker.db");
        let conn = open_and_migrate(&path).expect("migrate");
        test(&conn);
        std::env::remove_var(CONFIG_DIR);
    }

    #[test]
    fn upsert_get_list_roundtrip() {
        with_conn(|conn| {
            upsert_offer(conn, "sandbox.exec", "0.1.0", "core").expect("upsert");
            upsert_offer(conn, "llm.chat", "0.1.0", "core").expect("upsert");
            upsert_offer(conn, "llm.chat", "0.2.0", "core").expect("replace");

            let chat = get_offer(conn, "llm.chat").expect("get").expect("some");
            assert_eq!(chat.version, "0.2.0");
            assert_eq!(chat.origin, "core");

            let listed = list_offers(conn).expect("list");
            assert_eq!(listed.len(), 2);
            assert_eq!(listed[0].offer_id, "llm.chat");
            assert_eq!(listed[1].offer_id, "sandbox.exec");

            assert!(get_offer(conn, "missing.offer").expect("get").is_none());
        });
    }
}
