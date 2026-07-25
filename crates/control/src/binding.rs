//! In-memory bindings with TTL (`bind` / `unbind`).

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use serde_json::Value;
use types::{BindingId, ErrorCode, OfferId};

use crate::principal::Principal;

/// Parameters for creating a binding.
#[derive(Clone, Debug)]
pub struct BindRequest {
    pub offer_id: OfferId,
    pub principal: Principal,
    pub policy_json: Value,
    pub ttl: Duration,
}

/// Frozen binding metadata (policy snapshot for the TTL window).
#[derive(Clone, Debug, PartialEq)]
pub struct BindingRecord {
    pub binding_id: BindingId,
    pub offer_id: OfferId,
    pub principal: Principal,
    pub policy_json: Value,
    pub expires_at: SystemTime,
}

impl BindingRecord {
    #[must_use]
    pub fn is_expired(&self, now: SystemTime) -> bool {
        now >= self.expires_at
    }
}

/// Process-local binding table.
#[derive(Debug, Default)]
pub struct BindingStore {
    bindings: HashMap<BindingId, BindingRecord>,
}

impl BindingStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a binding and freeze policy for `ttl`.
    pub fn bind(&mut self, req: BindRequest) -> BindingRecord {
        let binding_id = BindingId::new();
        let now = SystemTime::now();
        let expires_at = now.checked_add(req.ttl).unwrap_or(now);
        let record = BindingRecord {
            binding_id,
            offer_id: req.offer_id,
            principal: req.principal,
            policy_json: req.policy_json,
            expires_at,
        };
        self.bindings.insert(binding_id, record.clone());
        record
    }

    /// Fetch a live (non-expired) binding.
    ///
    /// # Errors
    /// Returns [`ErrorCode::BindingExpired`] when missing or past TTL.
    pub fn get(&self, binding_id: BindingId) -> Result<&BindingRecord, ErrorCode> {
        self.get_at(binding_id, SystemTime::now())
    }

    fn get_at(&self, binding_id: BindingId, now: SystemTime) -> Result<&BindingRecord, ErrorCode> {
        let Some(record) = self.bindings.get(&binding_id) else {
            return Err(ErrorCode::BindingExpired);
        };
        if record.is_expired(now) {
            return Err(ErrorCode::BindingExpired);
        }
        Ok(record)
    }

    /// Remove a binding (idempotent finalization for callers that ignore miss).
    ///
    /// # Errors
    /// Returns [`ErrorCode::BindingExpired`] when the id is unknown.
    pub fn unbind(&mut self, binding_id: BindingId) -> Result<BindingRecord, ErrorCode> {
        self.bindings
            .remove(&binding_id)
            .ok_or(ErrorCode::BindingExpired)
    }

    /// Drop expired rows; returns how many were removed.
    pub fn purge_expired(&mut self) -> usize {
        let now = SystemTime::now();
        let before = self.bindings.len();
        self.bindings.retain(|_, r| !r.is_expired(now));
        before - self.bindings.len()
    }

    /// Live (non-expired) bindings in arbitrary map order.
    #[must_use]
    pub fn list(&self) -> Vec<&BindingRecord> {
        let now = SystemTime::now();
        self.bindings
            .values()
            .filter(|r| !r.is_expired(now))
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_req(ttl: Duration) -> BindRequest {
        BindRequest {
            offer_id: OfferId::new("llm.chat").expect("valid"),
            principal: Principal::local(),
            policy_json: json!({"caps": {"max_tokens": 16}}),
            ttl,
        }
    }

    #[test]
    fn bind_get_unbind_roundtrip() {
        let mut store = BindingStore::new();
        let record = store.bind(sample_req(Duration::from_secs(60)));
        let live = store.get(record.binding_id).expect("live");
        assert_eq!(live.offer_id.as_str(), "llm.chat");
        assert_eq!(live.principal, Principal::local());
        assert_eq!(live.principal.kind.as_str(), "local");

        let removed = store.unbind(record.binding_id).expect("unbind");
        assert_eq!(removed.binding_id, record.binding_id);
        assert_eq!(
            store.unbind(record.binding_id),
            Err(ErrorCode::BindingExpired)
        );
        assert!(store.is_empty());
    }

    #[test]
    fn zero_ttl_is_immediately_expired() {
        let mut store = BindingStore::new();
        let record = store.bind(sample_req(Duration::ZERO));
        // Expires at "now"; get_at with a later instant must fail.
        let later = record.expires_at + Duration::from_millis(1);
        assert_eq!(
            store.get_at(record.binding_id, later),
            Err(ErrorCode::BindingExpired)
        );
    }

    #[test]
    fn purge_expired_removes_stale_rows() {
        let mut store = BindingStore::new();
        let stale = store.bind(sample_req(Duration::ZERO));
        let live = store.bind(sample_req(Duration::from_secs(3600)));
        // Force stale by checking purge after ensuring zero-ttl is expired.
        let _ = store.get_at(stale.binding_id, stale.expires_at + Duration::from_secs(1));
        let removed = store.purge_expired();
        assert!(removed >= 1);
        assert!(store.get(live.binding_id).is_ok());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn api_key_principal_roundtrip() {
        let mut store = BindingStore::new();
        let record = store.bind(BindRequest {
            offer_id: OfferId::new("llm.chat").expect("valid"),
            principal: Principal::from_bind_arg("api_key:alice"),
            policy_json: json!({}),
            ttl: Duration::from_secs(30),
        });
        assert_eq!(record.principal.id, "alice");
        assert_eq!(
            record.principal.kind,
            crate::principal::PrincipalKind::ApiKey
        );
    }
}
