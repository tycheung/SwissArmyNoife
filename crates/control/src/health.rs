//! `broker.health` — control-plane liveness snapshot (`sak060`).

use std::sync::Arc;

use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::{CatalogEntry, Offer};

/// Snapshot provider (MCP injects live stores).
pub trait HealthSnapshot: Send + Sync {
    fn snapshot(&self) -> Value;
}

/// Static/empty snapshot for library tests.
#[derive(Clone, Debug, Default)]
pub struct EmptyHealthSnapshot;

impl HealthSnapshot for EmptyHealthSnapshot {
    fn snapshot(&self) -> Value {
        json!({
            "ok": true,
            "offers": 0,
            "bindings": 0,
            "policy": "ambient",
        })
    }
}

/// Control-plane health offer.
pub struct BrokerHealthOffer {
    entry: CatalogEntry,
    snap: Arc<dyn HealthSnapshot>,
}

impl BrokerHealthOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(snap: Arc<dyn HealthSnapshot>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("broker.health", "0.1.0")?,
            snap,
        })
    }

    /// Empty snapshot offer (unit tests).
    ///
    /// # Errors
    /// Invalid catalog id.
    pub fn empty() -> Result<Self, ErrorCode> {
        Self::new(Arc::new(EmptyHealthSnapshot))
    }

    /// Current health JSON (no invoke / binding required).
    #[must_use]
    pub fn snapshot_json(&self) -> Value {
        self.snap.snapshot()
    }
}

impl Offer for BrokerHealthOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-broker.health".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        InvokeResp::ok(invoke_id, self.snap.snapshot())
    }

    async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn health_ok() {
        let offer = BrokerHealthOffer::empty().unwrap();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::from_uuid(Uuid::nil()),
                args: json!({}),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => assert_eq!(result["ok"], true),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn empty_snapshot_defaults() {
        let snap = EmptyHealthSnapshot.snapshot();
        assert_eq!(snap["ok"], true);
        assert_eq!(snap["offers"], 0);
        assert_eq!(snap["bindings"], 0);
        assert_eq!(snap["policy"], "ambient");
    }

    #[test]
    fn empty_offer_snapshot_json_matches_provider() {
        let offer = BrokerHealthOffer::empty().expect("valid");
        assert_eq!(offer.snapshot_json(), EmptyHealthSnapshot.snapshot());
    }
}
