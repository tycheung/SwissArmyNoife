//! `capacity.pressure` offer (`sak274-b`).

use std::sync::Arc;
use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde_json::Value;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::governor::GovernorBudget;
use crate::pressure::{admit_or_err, sample_pressure};
use crate::probe::HardwareProbe;

/// Sample pressure vs frozen governor budget.
pub struct CapacityPressureOffer {
    entry: CatalogEntry,
    probe: Arc<dyn HardwareProbe>,
    budget: Mutex<GovernorBudget>,
}

impl CapacityPressureOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(probe: Arc<dyn HardwareProbe>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("capacity.pressure", "0.1.0")?,
            probe,
            budget: Mutex::new(GovernorBudget::default()),
        })
    }
}

impl Offer for CapacityPressureOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-capacity.pressure".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let mut budget = self.budget.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *budget = GovernorBudget::from_policy(&params);
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
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
        // Optional per-invoke budget override.
        let budget = if req.args.get("capacity").is_some() || req.args.get("max_ram_mb").is_some() {
            GovernorBudget::from_policy(&req.args)
        } else {
            budget
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
        let sample = sample_pressure(&snap, &budget);
        if let Err(code) = admit_or_err(&sample) {
            return InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code,
                message: sample.reason.clone(),
            };
        }
        match serde_json::to_value(&sample) {
            Ok(v) => InvokeResp::ok(invoke_id, v),
            Err(e) => InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code: ErrorCode::SchemaInvalid,
                message: format!("serialize: {e}"),
            },
        }
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
    use crate::probe::{FakeProbe, HardwareSnapshot};
    use serde_json::json;
    use types::InvokeId;

    #[tokio::test]
    async fn pressure_deny_returns_budget_exhausted() {
        let fake = FakeProbe {
            snapshot: HardwareSnapshot {
                total_ram_mb: 4_096,
                available_ram_mb: 100,
                cpu_logical: 4,
                cpu_usage_pct: 10.0,
                total_vram_mb: 0,
                available_vram_mb: 0,
                source: "fake".into(),
            },
        };
        let offer = CapacityPressureOffer::new(Arc::new(fake)).expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({ "capacity": { "min_free_ram_mb": 2048, "max_ram_mb": 512 } }),
            )
            .await
            .unwrap();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error { code, .. } => assert_eq!(code, ErrorCode::BudgetExhausted),
            other => panic!("expected deny, got {other:?}"),
        }
    }
}
