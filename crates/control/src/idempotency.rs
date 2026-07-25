//! Idempotency keys for bind (`sak061-a`).

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use serde_json::Value;
use sha2::{Digest, Sha256};
use types::{BindingId, ErrorCode, OfferId};

use crate::principal::Principal;

/// Cached idempotency outcome keyed by client token.
#[derive(Clone, Debug, PartialEq)]
struct IdempotencyEntry {
    fingerprint: String,
    result: IdempotencyResult,
    expires_at: SystemTime,
}

#[derive(Clone, Debug, PartialEq)]
enum IdempotencyResult {
    Binding(BindingId),
    Provision(String),
}

/// In-memory idempotency table (process-local).
#[derive(Debug)]
pub struct IdempotencyStore {
    entries: HashMap<String, IdempotencyEntry>,
    ttl: Duration,
}

impl IdempotencyStore {
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    /// Default TTL for bind idempotency (1 hour).
    #[must_use]
    pub fn default_bind() -> Self {
        Self::new(Duration::from_secs(3600))
    }

    /// Stable hash of offer + principal + canonical policy JSON.
    #[must_use]
    pub fn bind_fingerprint(
        offer_id: &OfferId,
        principal: &Principal,
        policy_json: &Value,
    ) -> String {
        let canonical = serde_json::json!({
            "offer_id": offer_id.as_str(),
            "principal_id": principal.id,
            "policy": policy_json,
        });
        let bytes = serde_json::to_string(&canonical).unwrap_or_default();
        format!("{:x}", Sha256::digest(bytes.as_bytes()))
    }

    /// Lookup an existing binding id for replay.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the key was used with a different fingerprint.
    pub fn lookup(
        &mut self,
        key: &str,
        fingerprint: &str,
        now: SystemTime,
    ) -> Result<Option<BindingId>, ErrorCode> {
        self.purge_expired(now);
        if let Some(entry) = self.entries.get(key) {
            if entry.fingerprint != fingerprint {
                return Err(ErrorCode::SchemaInvalid);
            }
            if now < entry.expires_at {
                return Ok(match entry.result {
                    IdempotencyResult::Binding(id) => Some(id),
                    IdempotencyResult::Provision(_) => None,
                });
            }
            self.entries.remove(key);
        }
        Ok(None)
    }

    /// Store a fresh bind outcome for later replay.
    pub fn record(&mut self, key: &str, fingerprint: &str, binding_id: BindingId, now: SystemTime) {
        let expires_at = now.checked_add(self.ttl).unwrap_or(now);
        self.entries.insert(
            key.to_owned(),
            IdempotencyEntry {
                fingerprint: fingerprint.to_owned(),
                result: IdempotencyResult::Binding(binding_id),
                expires_at,
            },
        );
    }

    /// Stable hash of offer id for provision replay.
    #[must_use]
    pub fn provision_fingerprint(offer_id: &OfferId) -> String {
        let canonical = serde_json::json!({ "offer_id": offer_id.as_str() });
        let bytes = serde_json::to_string(&canonical).unwrap_or_default();
        format!("{:x}", Sha256::digest(bytes.as_bytes()))
    }

    fn provision_key(key: &str) -> String {
        format!("provision:{key}")
    }

    /// Lookup an existing resource id for provision replay.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the key was used with a different fingerprint.
    pub fn lookup_provision(
        &mut self,
        key: &str,
        fingerprint: &str,
        now: SystemTime,
    ) -> Result<Option<String>, ErrorCode> {
        self.purge_expired(now);
        let namespaced = Self::provision_key(key);
        if let Some(entry) = self.entries.get(&namespaced) {
            if entry.fingerprint != fingerprint {
                return Err(ErrorCode::SchemaInvalid);
            }
            if now < entry.expires_at {
                return Ok(match &entry.result {
                    IdempotencyResult::Provision(resource_id) => Some(resource_id.clone()),
                    IdempotencyResult::Binding(_) => None,
                });
            }
            self.entries.remove(&namespaced);
        }
        Ok(None)
    }

    /// Store a fresh provision outcome for later replay.
    pub fn record_provision(
        &mut self,
        key: &str,
        fingerprint: &str,
        resource_id: &str,
        now: SystemTime,
    ) {
        let expires_at = now.checked_add(self.ttl).unwrap_or(now);
        self.entries.insert(
            Self::provision_key(key),
            IdempotencyEntry {
                fingerprint: fingerprint.to_owned(),
                result: IdempotencyResult::Provision(resource_id.to_owned()),
                expires_at,
            },
        );
    }

    fn purge_expired(&mut self, now: SystemTime) {
        self.entries.retain(|_, e| now < e.expires_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replay_same_fingerprint_returns_binding_id() {
        let mut store = IdempotencyStore::default_bind();
        let offer = OfferId::new("llm.chat").expect("valid");
        let principal = Principal::local();
        let policy = json!({});
        let fp = IdempotencyStore::bind_fingerprint(&offer, &principal, &policy);
        let id = BindingId::new();
        let now = SystemTime::now();
        store.record("k1", &fp, id, now);
        assert_eq!(store.lookup("k1", &fp, now).expect("replay"), Some(id));
    }

    #[test]
    fn conflicting_fingerprint_is_schema_invalid() {
        let mut store = IdempotencyStore::default_bind();
        let offer = OfferId::new("llm.chat").expect("valid");
        let principal = Principal::local();
        let fp1 = IdempotencyStore::bind_fingerprint(&offer, &principal, &json!({}));
        let fp2 = IdempotencyStore::bind_fingerprint(&offer, &principal, &json!({"x": 1}));
        let now = SystemTime::now();
        store.record("k1", &fp1, BindingId::new(), now);
        assert_eq!(store.lookup("k1", &fp2, now), Err(ErrorCode::SchemaInvalid));
    }

    #[test]
    fn provision_replay_same_fingerprint_returns_resource_id() {
        let mut store = IdempotencyStore::default_bind();
        let offer = OfferId::new("llm.chat").expect("valid");
        let fp = IdempotencyStore::provision_fingerprint(&offer);
        let now = SystemTime::now();
        store.record_provision("p1", &fp, "res-llm.chat-abc", now);
        assert_eq!(
            store.lookup_provision("p1", &fp, now).expect("replay"),
            Some("res-llm.chat-abc".into())
        );
    }

    #[test]
    fn provision_conflicting_fingerprint_is_schema_invalid() {
        let mut store = IdempotencyStore::default_bind();
        let chat = OfferId::new("llm.chat").expect("valid");
        let exec = OfferId::new("sandbox.exec").expect("valid");
        let fp1 = IdempotencyStore::provision_fingerprint(&chat);
        let fp2 = IdempotencyStore::provision_fingerprint(&exec);
        let now = SystemTime::now();
        store.record_provision("p1", &fp1, "res-a", now);
        assert_eq!(
            store.lookup_provision("p1", &fp2, now),
            Err(ErrorCode::SchemaInvalid)
        );
    }

    #[test]
    fn expired_entry_is_purged_on_lookup() {
        let mut store = IdempotencyStore::new(Duration::from_secs(1));
        let offer = OfferId::new("llm.chat").expect("valid");
        let principal = Principal::local();
        let fp = IdempotencyStore::bind_fingerprint(&offer, &principal, &json!({}));
        let id = BindingId::new();
        let start = SystemTime::UNIX_EPOCH;
        store.record("k-exp", &fp, id, start);
        let after_ttl = start + Duration::from_secs(2);
        assert_eq!(store.lookup("k-exp", &fp, after_ttl).expect("miss"), None);
    }
}
