//! Coverage floor smoke (`sak063-a` / `sak063-c`) — lightly-covered paths in one integration file.

use std::time::SystemTime;

use control::{
    resolve_policy, ApiKeyStore, AuditLog, AuditStatus, BrokerHealthOffer, EmptyHealthSnapshot,
    HealthSnapshot, IdempotencyStore, MeterSnapshot, Offer, RateLimiter,
};
use serde_json::json;
use types::{BindingId, ErrorCode, InvokeId, InvokeResp, OfferId};
use uuid::Uuid;

#[test]
fn smoke_soft_delete_hides_from_active_list() {
    let mut log = AuditLog::new();
    let invoke_id = InvokeId::from_uuid(Uuid::from_u128(42));
    let binding_id = types::BindingId::from_uuid(Uuid::from_u128(1));
    let offer_id = OfferId::new("llm.chat").expect("valid");
    log.record_invoke(
        invoke_id,
        binding_id,
        offer_id,
        &json!({}),
        &InvokeResp::ok(invoke_id, json!({})),
    );
    assert_eq!(log.list_active().len(), 1);
    assert!(log.soft_delete(invoke_id, SystemTime::now()));
    assert!(log.list_active().is_empty());
    assert_eq!(log.events()[0].status, AuditStatus::Ok);
}

#[test]
fn smoke_idempotency_conflict_is_schema_invalid() {
    let mut store = IdempotencyStore::default_bind();
    let offer = OfferId::new("sandbox.exec").expect("valid");
    let principal = control::Principal::local();
    let fp_a = IdempotencyStore::bind_fingerprint(&offer, &principal, &json!({}));
    let fp_b = IdempotencyStore::bind_fingerprint(&offer, &principal, &json!({"tier": "strict"}));
    let now = SystemTime::now();
    store.record("idem-key", &fp_a, BindingId::new(), now);
    assert_eq!(
        store.lookup("idem-key", &fp_b, now),
        Err(ErrorCode::SchemaInvalid)
    );
}

#[test]
fn smoke_unknown_policy_template_is_invalid() {
    assert_eq!(
        resolve_policy(Some("does-not-exist"), None),
        Err(ErrorCode::SchemaInvalid)
    );
}

#[test]
fn smoke_empty_health_snapshot_shape() {
    let snap = EmptyHealthSnapshot.snapshot();
    assert_eq!(snap["ok"], true);
    assert_eq!(snap["offers"], 0);
    assert_eq!(snap["bindings"], 0);
    assert_eq!(snap["policy"], "ambient");
}

#[test]
fn smoke_policy_template_mutual_exclusion() {
    assert_eq!(
        resolve_policy(Some("local-dev"), Some(json!({}))),
        Err(ErrorCode::SchemaInvalid)
    );
}

#[test]
fn smoke_meter_jsonl_non_empty() {
    let text = MeterSnapshot::new(1, 2, 3).to_jsonl();
    assert!(!text.is_empty());
    assert_eq!(text.lines().count(), 3);
}

#[test]
fn smoke_idempotency_provision_namespace() {
    let mut store = IdempotencyStore::default_bind();
    let offer = OfferId::new("llm.chat").expect("valid");
    let fp = IdempotencyStore::provision_fingerprint(&offer);
    let now = SystemTime::now();
    store.record_provision("idem-provision", &fp, "res-llm.chat-1", now);
    assert_eq!(
        store
            .lookup_provision("idem-provision", &fp, now)
            .expect("replay"),
        Some("res-llm.chat-1".into())
    );
}

#[test]
fn smoke_purge_before_soft_deleted() {
    let mut log = AuditLog::new();
    let invoke_id = InvokeId::from_uuid(Uuid::from_u128(99));
    let binding_id = BindingId::from_uuid(Uuid::from_u128(2));
    let offer_id = OfferId::new("sandbox.exec").expect("valid");
    log.record_invoke(
        invoke_id,
        binding_id,
        offer_id,
        &json!({}),
        &InvokeResp::ok(invoke_id, json!({})),
    );
    let old = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10);
    assert!(log.soft_delete(invoke_id, old));
    let cutoff = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(20);
    assert_eq!(log.purge_before(cutoff), 1);
    assert!(log.is_empty());
}

#[test]
fn smoke_rate_limit_denies_after_burst() {
    let mut lim = RateLimiter::with_per_min(2.0);
    lim.check("principal-a").expect("first");
    lim.check("principal-a").expect("second");
    assert_eq!(lim.check("principal-a"), Err(ErrorCode::PolicyDenied));
}

#[test]
fn smoke_policy_template_strict_egress_happy_path() {
    let policy = resolve_policy(Some("strict-egress"), None).expect("template");
    assert_eq!(policy["egress"]["allow_hosts"], json!([]));
}

#[test]
fn smoke_api_key_mint_get_and_export() {
    let store = ApiKeyStore::new();
    let (info, secret) = store.mint("publisher").expect("mint");
    let looked_up = store.get(&info.key_id).expect("get");
    assert_eq!(looked_up.principal_id, "publisher");
    let rows = store.export_rows().expect("export");
    assert_eq!(rows.len(), 1);
    assert_eq!(store.verify(&secret).expect("verify").id, "publisher");
}

#[test]
fn smoke_broker_health_catalog_version() {
    let offer = BrokerHealthOffer::empty().expect("offer");
    assert_eq!(offer.catalog_entry().version, "0.1.0");
    assert_eq!(offer.catalog_entry().id.as_str(), "broker.health");
}
