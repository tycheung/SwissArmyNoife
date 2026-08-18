//! `SQLite` hydrate/persist for MCP bindings and audit (`sak572` / `refactor:mcp-persist`).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use control::{AuditEvent, AuditLog, AuditStatus, BindingRecord, BindingStore, Principal};
use serde_json::json;
use types::{BindingId, ErrorCode, InvokeId, OfferId};

use crate::util::expires_unix;

/// `SQLite` is the default persist backend; Postgres is opt-in (`sak572-e`).
pub(crate) fn persist_backend_kind() -> &'static str {
    #[cfg(feature = "postgres")]
    {
        if persist_postgres::postgres_enabled() && persist_postgres::pg_url_from_env().is_some() {
            return "postgres";
        }
    }
    "sqlite"
}

pub(crate) fn log_persist_backend() {
    let kind = persist_backend_kind();
    #[cfg(feature = "postgres")]
    if kind == "postgres" {
        match persist_postgres::try_open_from_env() {
            Ok(Some(_)) => tracing::info!("mcp persist backend: postgres"),
            Ok(None) => tracing::info!("mcp persist backend: sqlite"),
            Err(e) => tracing::warn!(
                error = %e,
                "postgres persist open failed; SQLite default unchanged"
            ),
        }
        return;
    }
    tracing::debug!("mcp persist backend: {kind}");
}

pub(crate) fn load_bindings() -> BindingStore {
    let mut store = BindingStore::new();
    let Ok(conn) = persist_sqlite::open_default() else {
        return store;
    };
    let Ok(rows) = persist_sqlite::list_bindings(&conn) else {
        return store;
    };
    let now = SystemTime::now();
    for row in rows {
        let Some(rec) = record_from_row(&row) else {
            continue;
        };
        if rec.is_expired(now) {
            continue;
        }
        store.insert_record(rec);
    }
    store
}

pub(crate) fn load_audit() -> AuditLog {
    let mut log = AuditLog::new();
    let Ok(conn) = persist_sqlite::open_default() else {
        return log;
    };
    let Ok(rows) = persist_sqlite::list_audit(&conn) else {
        return log;
    };
    for row in rows {
        if let Some(ev) = audit_from_row(&row) {
            log.append(ev);
        }
    }
    log
}

pub(crate) fn persist_audit(event: &AuditEvent) {
    let Ok(conn) = persist_sqlite::open_default() else {
        tracing::warn!("audit persist: open_default failed");
        return;
    };
    let created_at_unix = i64::try_from(expires_unix(event.created_at)).unwrap_or(0);
    let detail_json = serde_json::to_string(&event.detail).unwrap_or_else(|_| "{}".into());
    let row = persist_sqlite::AuditRow {
        invoke_id: event.invoke_id.to_string(),
        binding_id: event.binding_id.to_string(),
        offer_id: event.offer_id.as_str().to_string(),
        status: event.status.as_str().to_string(),
        code: event.code.map(|c| c.as_str().to_string()),
        detail_json,
        created_at_unix,
    };
    if let Err(e) = persist_sqlite::put_audit(&conn, &row) {
        tracing::warn!("audit persist: {e}");
    }
}

fn audit_from_row(row: &persist_sqlite::AuditRow) -> Option<AuditEvent> {
    let invoke_id = InvokeId::from_uuid(uuid::Uuid::parse_str(&row.invoke_id).ok()?);
    let binding_id = BindingId::from_uuid(uuid::Uuid::parse_str(&row.binding_id).ok()?);
    let status = if row.status == "error" {
        AuditStatus::Error
    } else {
        AuditStatus::Ok
    };
    let code = row
        .code
        .as_deref()
        .and_then(|raw| serde_json::from_str::<ErrorCode>(&format!("\"{raw}\"")).ok());
    Some(AuditEvent {
        invoke_id,
        binding_id,
        offer_id: OfferId::new(&row.offer_id).ok()?,
        status,
        code,
        detail: serde_json::from_str(&row.detail_json).unwrap_or_else(|_| json!({})),
        created_at: UNIX_EPOCH + Duration::from_secs(u64::try_from(row.created_at_unix).ok()?),
        deleted_at: None,
    })
}

pub(crate) fn persist_binding(record: &BindingRecord) {
    let Ok(conn) = persist_sqlite::open_default() else {
        tracing::warn!("binding persist: open_default failed");
        return;
    };
    let principal = if record.principal.kind.as_str() == "api_key" {
        format!("api_key:{}", record.principal.id)
    } else {
        record.principal.id.clone()
    };
    let policy_json = serde_json::to_string(&record.policy_json).unwrap_or_else(|_| "{}".into());
    let expires_at_unix = i64::try_from(expires_unix(record.expires_at)).unwrap_or(i64::MAX);
    let row = persist_sqlite::BindingRow {
        binding_id: record.binding_id.to_string(),
        offer_id: record.offer_id.as_str().to_string(),
        principal,
        policy_json,
        expires_at_unix,
    };
    if let Err(e) = persist_sqlite::put_binding(&conn, &row) {
        tracing::warn!("binding persist: {e}");
    }
}

pub(crate) fn forget_binding(id: BindingId) {
    let Ok(conn) = persist_sqlite::open_default() else {
        return;
    };
    let _ = persist_sqlite::delete_binding(&conn, &id.to_string());
}

fn record_from_row(row: &persist_sqlite::BindingRow) -> Option<BindingRecord> {
    let uuid = uuid::Uuid::parse_str(&row.binding_id).ok()?;
    let expires_at = UNIX_EPOCH + Duration::from_secs(u64::try_from(row.expires_at_unix).ok()?);
    Some(BindingRecord {
        binding_id: BindingId::from_uuid(uuid),
        offer_id: OfferId::new(&row.offer_id).ok()?,
        principal: Principal::from_bind_arg(&row.principal),
        policy_json: serde_json::from_str(&row.policy_json).ok()?,
        expires_at,
    })
}
