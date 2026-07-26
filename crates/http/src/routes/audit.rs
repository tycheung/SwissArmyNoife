//! Audit query endpoints (`sak528-a`) — redacted invoke events.

use std::time::{Duration, UNIX_EPOCH};

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use control::AuditEvent;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct AuditQuery {
    #[serde(default)]
    offer_id: Option<String>,
    /// Unix seconds; inclusive lower bound on `created_at`.
    #[serde(default)]
    since: Option<u64>,
}

fn event_json(ev: &AuditEvent) -> Value {
    let created_at = ev
        .created_at
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    json!({
        "invoke_id": ev.invoke_id.to_string(),
        "binding_id": ev.binding_id.to_string(),
        "offer_id": ev.offer_id.as_str(),
        "status": ev.status.as_str(),
        "code": ev.code.map(|c| c.as_str().to_owned()),
        "detail": ev.detail,
        "created_at": created_at,
    })
}

async fn list_audit(State(state): State<AppState>, Query(q): Query<AuditQuery>) -> Json<Value> {
    let since = q.since.map(|s| UNIX_EPOCH + Duration::from_secs(s));
    let audit = state.audit.lock().expect("audit lock");
    let events: Vec<_> = audit
        .query(q.offer_id.as_deref(), since)
        .into_iter()
        .map(event_json)
        .collect();
    Json(json!({ "events": events }))
}

pub fn audit_router() -> Router<AppState> {
    Router::new().route("/v1/sak/audit", get(list_audit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use control::{AuditLog, AuditStatus};
    use std::time::SystemTime;
    use types::{BindingId, InvokeId, OfferId};
    use uuid::Uuid;

    #[test]
    fn event_json_keeps_redacted_detail() {
        let ev = AuditEvent {
            invoke_id: InvokeId::from_uuid(Uuid::nil()),
            binding_id: BindingId::from_uuid(Uuid::from_u128(1)),
            offer_id: OfferId::new("llm.chat").expect("id"),
            status: AuditStatus::Ok,
            code: None,
            detail: json!({ "args": { "api_key": "[REDACTED]" } }),
            created_at: SystemTime::UNIX_EPOCH,
            deleted_at: None,
        };
        let v = event_json(&ev);
        assert_eq!(v["detail"]["args"]["api_key"], "[REDACTED]");
        let _ = AuditLog::new();
    }
}
