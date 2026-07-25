//! `capacity.probe` offer (`sak274-a`).

use std::sync::Arc;

use control::{CatalogEntry, Offer};
use serde_json::Value;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::probe::HardwareProbe;

/// Return a fresh hardware snapshot.
pub struct CapacityProbeOffer {
    entry: CatalogEntry,
    probe: Arc<dyn HardwareProbe>,
}

impl CapacityProbeOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(probe: Arc<dyn HardwareProbe>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("capacity.probe", "0.1.0")?,
            probe,
        })
    }
}

impl Offer for CapacityProbeOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-capacity.probe".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        let snap = match self.probe.probe() {
            Ok(s) => s,
            Err(code) => {
                return InvokeResp::Error {
                    invoke_id: Some(invoke_id),
                    code,
                    message: "hardware probe unavailable".into(),
                };
            }
        };
        match serde_json::to_value(&snap) {
            Ok(v) => InvokeResp::ok(invoke_id, v),
            Err(e) => InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code: ErrorCode::SchemaInvalid,
                message: format!("serialize snapshot: {e}"),
            },
        }
    }

    async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        let _ = self.probe.probe()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::FakeProbe;
    use serde_json::json;
    use types::InvokeId;

    #[tokio::test]
    async fn probe_invoke_returns_snapshot() {
        let offer = CapacityProbeOffer::new(Arc::new(FakeProbe::typical_laptop())).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["source"], "fake");
                assert!(result["total_ram_mb"].as_u64().unwrap() > 0);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
