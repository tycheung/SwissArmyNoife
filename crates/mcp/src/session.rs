//! Multi-offer session helper — auto-bind a pack (`sak114-a`).

use std::time::Duration;

use control::{BindRequest, BindingStore, PolicyEngine, Principal};
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, OfferId};

/// Bind several offers with the same principal / TTL / base policy.
///
/// # Errors
/// Invalid offer id or policy deny.
pub fn bind_pack(
    store: &mut BindingStore,
    policy: &PolicyEngine,
    offer_ids: &[String],
    principal: &Principal,
    ttl_secs: u64,
    base_policy: &Value,
) -> Result<Vec<(String, BindingId)>, ErrorCode> {
    let mut out = Vec::with_capacity(offer_ids.len());
    for raw in offer_ids {
        let offer_id = OfferId::new(raw.clone())?;
        let mut policy_json = base_policy.clone();
        if let Some(obj) = policy_json.as_object_mut() {
            obj.entry("principal".to_string())
                .or_insert_with(|| json!(principal.as_str()));
            obj.entry("principal_kind".to_string())
                .or_insert_with(|| json!(principal.kind.as_str()));
        }
        policy.check(principal.as_str(), &offer_id)?;
        let record = store.bind(BindRequest {
            offer_id,
            principal: principal.clone(),
            policy_json,
            ttl: Duration::from_secs(ttl_secs.max(1)),
        });
        out.push((raw.clone(), record.binding_id));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_two_offers() {
        let mut store = BindingStore::new();
        let policy = PolicyEngine::ambient();
        let pack = bind_pack(
            &mut store,
            &policy,
            &["llm.chat".into(), "sandbox.exec".into()],
            &Principal::local(),
            60,
            &json!({}),
        )
        .unwrap();
        assert_eq!(pack.len(), 2);
        assert!(store.get(pack[0].1).is_ok());
    }
}
