//! Live store round-trip tests (`sak070` Phase B). Gated on `SAK_PG_URL`.

use super::{AuditEventRow, AuditStore, BindingRow, BindingStore, CatalogStore, PersistPortError};
use crate::backend::try_open_from_url_env;
use crate::env::test_lock;

fn unique(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos}")
}

#[test]
fn live_stores_roundtrip_when_sak_pg_url_set() {
    let _g = test_lock::lock();
    let backend = match try_open_from_url_env() {
        Ok(Some(b)) => b,
        Ok(None) => {
            eprintln!(
                "skip live_stores_roundtrip_when_sak_pg_url_set (no SAK_PG_URL/DATABASE_URL)"
            );
            return;
        }
        Err(e) => panic!("open backend: {e}"),
    };
    assert!(backend.pool.is_connected());

    let offer_id = unique("offer");
    backend
        .catalog
        .upsert_offer(&offer_id, "0.1.0", "core")
        .expect("upsert offer");
    let got = backend
        .catalog
        .get_offer(&offer_id)
        .expect("get")
        .expect("some");
    assert_eq!(got.version, "0.1.0");
    assert_eq!(got.origin, "core");
    assert!(backend
        .catalog
        .list_offers()
        .expect("list")
        .iter()
        .any(|r| r.offer_id == offer_id));

    let binding_id = unique("bind");
    backend
        .bindings
        .insert_binding(&BindingRow {
            binding_id: binding_id.clone(),
            offer_id: offer_id.clone(),
            created_at_unix: 1,
        })
        .expect("insert binding");
    let bind = backend
        .bindings
        .get_binding(&binding_id)
        .expect("get binding")
        .expect("some");
    assert_eq!(bind.offer_id, offer_id);
    assert!(bind.created_at_unix > 0);

    let event_id = unique("evt");
    backend
        .audit
        .append_event(&AuditEventRow {
            event_id: event_id.clone(),
            binding_id: binding_id.clone(),
            kind: "invoke".into(),
            recorded_at_unix: 1,
        })
        .expect("append");
    assert!(backend.audit.event_exists(&event_id).expect("exists"));
}

#[test]
fn try_open_from_env_none_without_backend_flag() {
    let _g = test_lock::lock();
    let p_back = std::env::var(crate::PERSIST_BACKEND_ENV).ok();
    let p_url = std::env::var(crate::PG_URL_ENV).ok();
    std::env::set_var(crate::PG_URL_ENV, "postgres://127.0.0.1:5432/sak");
    std::env::set_var(crate::PERSIST_BACKEND_ENV, "sqlite");
    let opened = crate::try_open_from_env().expect("no connect");
    assert!(opened.is_none());
    match p_back {
        Some(v) => std::env::set_var(crate::PERSIST_BACKEND_ENV, v),
        None => std::env::remove_var(crate::PERSIST_BACKEND_ENV),
    }
    match p_url {
        Some(v) => std::env::set_var(crate::PG_URL_ENV, v),
        None => std::env::remove_var(crate::PG_URL_ENV),
    }
}

#[test]
fn unimplemented_still_default_stub() {
    use super::UnimplementedCatalog;
    let err = UnimplementedCatalog
        .upsert_offer("x", "0.1.0", "core")
        .unwrap_err();
    assert_eq!(err, PersistPortError::NotImplemented);
}
