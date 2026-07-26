//! `llm.preflight` — reachability + optional fit ranks (`sak138` / `sak273-b`).

use std::sync::Arc;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// Pluggable fit ranking (implemented in MCP via `offer-capacity`, not a cross-offer dep).
pub trait FitAdvisor: Send + Sync {
    /// Rank candidates; return JSON array of `{id, score, fits, reason}`.
    fn rank(&self, candidates: &[PreflightCandidate]) -> Vec<Value>;
}

/// No-op advisor (preflight without hardware fit).
#[derive(Clone, Debug, Default)]
pub struct NoFitAdvisor;

impl FitAdvisor for NoFitAdvisor {
    fn rank(&self, _candidates: &[PreflightCandidate]) -> Vec<Value> {
        Vec::new()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PreflightCandidate {
    pub id: String,
    pub ram_mb: u64,
    #[serde(default)]
    pub vram_mb: u64,
}

/// Routing preflight offer.
pub struct LlmPreflightOffer {
    entry: CatalogEntry,
    fit: Arc<dyn FitAdvisor>,
    /// Providers considered reachable in this process (e.g. `echo`, `ollama`).
    reachable: Vec<String>,
}

impl LlmPreflightOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(fit: Arc<dyn FitAdvisor>, reachable: Vec<String>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("llm.preflight", "0.1.0")?,
            fit,
            reachable,
        })
    }
}

impl Offer for LlmPreflightOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-llm.preflight".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run(self, &req.args) {
            Ok(v) => InvokeResp::ok(invoke_id, v),
            Err((code, message)) => InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code,
                message,
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

#[derive(Debug, Deserialize)]
struct PreflightArgs {
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    candidates: Option<Vec<PreflightCandidate>>,
}

fn run(offer: &LlmPreflightOffer, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: PreflightArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("preflight args: {e}")))?;
    let provider = parsed.provider.unwrap_or_else(|| {
        offer
            .reachable
            .first()
            .cloned()
            .unwrap_or_else(|| "echo".into())
    });
    let reachable = offer
        .reachable
        .iter()
        .any(|p| p.eq_ignore_ascii_case(&provider));
    let ranks = parsed
        .candidates
        .as_deref()
        .map(|c| offer.fit.rank(c))
        .unwrap_or_default();
    let recommended = ranks
        .iter()
        .find(|r| r.get("fits").and_then(Value::as_bool) == Some(true))
        .and_then(|r| r.get("id").and_then(Value::as_str))
        .map(str::to_owned);
    Ok(json!({
        "provider": provider,
        "reachable": reachable,
        "reachable_providers": offer.reachable,
        "fit_ranks": ranks,
        "recommended_model": recommended,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    struct FixedFit;

    impl FitAdvisor for FixedFit {
        fn rank(&self, candidates: &[PreflightCandidate]) -> Vec<Value> {
            candidates
                .iter()
                .map(|c| {
                    #[allow(clippy::cast_precision_loss)]
                    let score = 1.0 / (c.ram_mb as f32 + 1.0);
                    json!({
                        "id": c.id,
                        "score": score,
                        "fits": c.ram_mb < 10_000,
                        "reason": "ok"
                    })
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn preflight_reports_reachability_and_fit() {
        let offer = LlmPreflightOffer::new(Arc::new(FixedFit), vec!["echo".into()]).unwrap();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "provider": "echo",
                    "candidates": [
                        { "id": "small", "ram_mb": 1000 },
                        { "id": "huge", "ram_mb": 99_000 }
                    ]
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["reachable"], true);
                assert_eq!(result["recommended_model"], "small");
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
