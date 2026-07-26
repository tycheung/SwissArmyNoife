//! Append-only invoke audit + JSON redaction helpers.

use std::time::SystemTime;

use serde_json::{json, Map, Value};
use types::{BindingId, ErrorCode, InvokeId, InvokeResp, OfferId};

/// One audited invoke decision (already redacted detail).
#[derive(Clone, Debug, PartialEq)]
pub struct AuditEvent {
    pub invoke_id: InvokeId,
    pub binding_id: BindingId,
    pub offer_id: OfferId,
    pub status: AuditStatus,
    pub code: Option<ErrorCode>,
    pub detail: Value,
    /// Append time (for query filters).
    pub created_at: SystemTime,
    /// Set by [`AuditLog::soft_delete`]; excluded from [`AuditLog::list_active`].
    pub deleted_at: Option<SystemTime>,
}

/// Wire-stable audit outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditStatus {
    Ok,
    Error,
}

impl AuditStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

/// Process-local append-only audit buffer.
#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, event: AuditEvent) {
        self.events.push(event);
    }

    /// Record an invoke response with redacted args in `detail`.
    pub fn record_invoke(
        &mut self,
        invoke_id: InvokeId,
        binding_id: BindingId,
        offer_id: OfferId,
        args: &Value,
        resp: &InvokeResp,
    ) {
        let (status, code) = match resp {
            InvokeResp::Ok { .. } => (AuditStatus::Ok, None),
            InvokeResp::Error { code, .. } => (AuditStatus::Error, Some(*code)),
        };
        self.append(AuditEvent {
            invoke_id,
            binding_id,
            offer_id,
            status,
            code,
            detail: json!({ "args": redact_json(args) }),
            created_at: SystemTime::now(),
            deleted_at: None,
        });
    }

    /// Mark an event soft-deleted by `invoke_id`. Returns false when not found.
    pub fn soft_delete(&mut self, invoke_id: InvokeId, at: SystemTime) -> bool {
        if let Some(ev) = self
            .events
            .iter_mut()
            .find(|e| e.invoke_id == invoke_id && e.deleted_at.is_none())
        {
            ev.deleted_at = Some(at);
            true
        } else {
            false
        }
    }

    /// Remove soft-deleted events whose `deleted_at` is before `cutoff`.
    pub fn purge_before(&mut self, cutoff: SystemTime) -> usize {
        let before = self.events.len();
        self.events.retain(|e| match e.deleted_at {
            None => true,
            Some(deleted) => deleted >= cutoff,
        });
        before - self.events.len()
    }

    /// Non-deleted events in append order.
    #[must_use]
    pub fn list_active(&self) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.deleted_at.is_none())
            .collect()
    }

    /// Active events filtered by optional `offer_id` and/or `since` (inclusive).
    #[must_use]
    pub fn query(&self, offer_id: Option<&str>, since: Option<SystemTime>) -> Vec<&AuditEvent> {
        self.list_active()
            .into_iter()
            .filter(|e| {
                if let Some(want) = offer_id {
                    if e.offer_id.as_str() != want {
                        return false;
                    }
                }
                if let Some(since) = since {
                    if e.created_at < since {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    #[must_use]
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Recursively redact values under secret-shaped object keys.
#[must_use]
pub fn redact_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(redact_map(map)),
        Value::Array(items) => Value::Array(items.iter().map(redact_json).collect()),
        other => other.clone(),
    }
}

fn redact_map(map: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for (key, val) in map {
        if is_sensitive_key(key) {
            out.insert(key.clone(), Value::String("[REDACTED]".into()));
        } else {
            out.insert(key.clone(), redact_json(val));
        }
    }
    out
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "password"
            | "token"
            | "secret"
            | "api_key"
            | "apikey"
            | "authorization"
            | "credential"
            | "private_key"
            | "access_token"
            | "refresh_token"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use types::{BindingId, OfferId};
    use uuid::Uuid;

    #[test]
    fn redact_json_masks_secret_keys_nested() {
        let raw = json!({
            "prompt": "hi",
            "api_key": "sk-live-secret",
            "nested": { "password": "hunter2", "n": 1 }
        });
        let redacted = redact_json(&raw);
        assert_eq!(redacted["prompt"], "hi");
        assert_eq!(redacted["api_key"], "[REDACTED]");
        assert_eq!(redacted["nested"]["password"], "[REDACTED]");
        assert_eq!(redacted["nested"]["n"], 1);
        assert!(!redacted.to_string().contains("sk-live"));
        assert!(!redacted.to_string().contains("hunter2"));
    }

    #[test]
    fn audit_log_records_redacted_invoke() {
        let mut log = AuditLog::new();
        let invoke_id = InvokeId::from_uuid(Uuid::nil());
        let binding_id = BindingId::from_uuid(Uuid::from_u128(1));
        let offer_id = OfferId::new("llm.chat").expect("valid");
        let args = json!({"token": "abc", "q": "ok"});
        let resp = InvokeResp::ok(invoke_id, json!({"text": "yo"}));

        log.record_invoke(invoke_id, binding_id, offer_id.clone(), &args, &resp);
        assert_eq!(log.len(), 1);
        let ev = &log.events()[0];
        assert_eq!(ev.status, AuditStatus::Ok);
        assert_eq!(ev.offer_id, offer_id);
        assert_eq!(ev.detail["args"]["token"], "[REDACTED]");
        assert_eq!(ev.detail["args"]["q"], "ok");
    }

    #[test]
    fn soft_delete_hides_from_list_active() {
        let mut log = AuditLog::new();
        let invoke_id = InvokeId::from_uuid(Uuid::nil());
        let binding_id = BindingId::from_uuid(Uuid::from_u128(1));
        let offer_id = OfferId::new("llm.chat").expect("valid");
        log.record_invoke(
            invoke_id,
            binding_id,
            offer_id,
            &json!({}),
            &InvokeResp::ok(invoke_id, json!({})),
        );
        assert_eq!(log.list_active().len(), 1);
        let at = SystemTime::now();
        assert!(log.soft_delete(invoke_id, at));
        assert!(log.list_active().is_empty());
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn query_filters_offer_and_since() {
        let mut log = AuditLog::new();
        let id = InvokeId::from_uuid(Uuid::nil());
        let binding_id = BindingId::from_uuid(Uuid::from_u128(1));
        let offer = OfferId::new("llm.chat").expect("valid");
        log.record_invoke(
            id,
            binding_id,
            offer,
            &json!({ "api_key": "sk-x" }),
            &InvokeResp::ok(id, json!({})),
        );
        assert_eq!(log.query(Some("llm.chat"), None).len(), 1);
        assert!(log.query(Some("other"), None).is_empty());
        let future = SystemTime::now() + std::time::Duration::from_secs(3600);
        assert!(log.query(None, Some(future)).is_empty());
        let ev = log.query(Some("llm.chat"), None)[0];
        assert_eq!(ev.detail["args"]["api_key"], "[REDACTED]");
    }

    #[test]
    fn purge_before_removes_old_soft_deleted_only() {
        let mut log = AuditLog::new();
        let id1 = InvokeId::from_uuid(Uuid::from_u128(1));
        let id2 = InvokeId::from_uuid(Uuid::from_u128(2));
        let binding_id = BindingId::from_uuid(Uuid::from_u128(9));
        let offer_id = OfferId::new("sandbox.exec").expect("valid");
        for id in [id1, id2] {
            log.record_invoke(
                id,
                binding_id,
                offer_id.clone(),
                &json!({}),
                &InvokeResp::ok(id, json!({})),
            );
        }
        let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100);
        let recent = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(200);
        assert!(log.soft_delete(id1, old));
        assert!(log.soft_delete(id2, recent));
        let cutoff = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(150);
        assert_eq!(log.purge_before(cutoff), 1);
        assert_eq!(log.len(), 1);
        assert_eq!(log.list_active().len(), 0);
        assert_eq!(log.events()[0].invoke_id, id2);
    }
}
