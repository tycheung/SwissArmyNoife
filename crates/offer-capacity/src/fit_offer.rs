//! `capacity.fit` offer (`sak274-c`).

use std::sync::Arc;
use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::fit::{rank_models, ModelCandidate};
use crate::governor::GovernorBudget;
use crate::probe::HardwareProbe;

/// Rank model candidates by fit.
pub struct CapacityFitOffer {
    entry: CatalogEntry,
    probe: Arc<dyn HardwareProbe>,
    budget: Mutex<GovernorBudget>,
}

impl CapacityFitOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(probe: Arc<dyn HardwareProbe>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("capacity.fit", "0.1.0")?,
            probe,
            budget: Mutex::new(GovernorBudget::default()),
        })
    }
}

impl Offer for CapacityFitOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-capacity.fit".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let mut budget = self.budget.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *budget = GovernorBudget::from_policy(&params);
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        let parsed: FitArgs = match serde_json::from_value(req.args.clone()) {
            Ok(a) => a,
            Err(e) => {
                return InvokeResp::Error {
                    invoke_id: Some(invoke_id),
                    code: ErrorCode::SchemaInvalid,
                    message: format!("fit args: {e}"),
                };
            }
        };
        let budget = match self.budget.lock() {
            Ok(b) => b.clone(),
            Err(_) => {
                return InvokeResp::Error {
                    invoke_id: Some(invoke_id),
                    code: ErrorCode::SchemaInvalid,
                    message: "budget lock".into(),
                };
            }
        };
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
        let ranks = rank_models(&snap, &budget, &parsed.candidates);
        InvokeResp::ok(
            invoke_id,
            json!({
                "ranks": ranks,
                "snapshot_source": snap.source,
            }),
        )
    }

    async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct FitArgs {
    candidates: Vec<ModelCandidate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::FakeProbe;
    use types::InvokeId;

    #[tokio::test]
    async fn fit_ranks_candidates() {
        let offer = CapacityFitOffer::new(Arc::new(FakeProbe::typical_laptop())).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "candidates": [
                        { "id": "a", "ram_mb": 1000 },
                        { "id": "b", "ram_mb": 500 }
                    ]
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                let ranks = result["ranks"].as_array().unwrap();
                assert_eq!(ranks[0]["id"], "b");
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
