//! `eval.*` generic check-runner host (not Nimbusware standards UI).
//!
//! Skeleton (`sak531-a`): `eval.run` Offer surface; runner wiring in sak531-b.

use control::{CatalogEntry, Offer};
use serde_json::Value;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// First-party `eval.run` offer (check runner wired in sak531-b).
pub struct EvalRunOffer {
    entry: CatalogEntry,
}

impl EvalRunOffer {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("eval.run", "0.1.0")?,
        })
    }
}

impl Offer for EvalRunOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-eval.run".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id;
        InvokeResp::Error {
            invoke_id,
            code: ErrorCode::SchemaInvalid,
            message: "eval.run runner not configured".into(),
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
    use types::InvokeId;

    #[tokio::test]
    async fn catalog_is_eval_run() {
        let offer = EvalRunOffer::new().expect("offer");
        assert_eq!(offer.catalog_entry().id.as_str(), "eval.run");
        assert_eq!(offer.catalog_entry().version, "0.1.0");
    }

    #[tokio::test]
    async fn invoke_reports_runner_not_configured() {
        let offer = EvalRunOffer::new().expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                invoke_id: Some(InvokeId::new()),
                args: Value::Object(Default::default()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error { code, message, .. } => {
                assert_eq!(code, ErrorCode::SchemaInvalid);
                assert!(message.contains("not configured"));
            }
            InvokeResp::Ok { .. } => panic!("expected error"),
        }
    }
}
