//! `llm.embed` offer: named-provider embed → JSON result.

use control::{CatalogEntry, Offer};
use provider_core::{EmbedRequest, ProviderError};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::EmbedProviders;

/// First-party `llm.embed` offer backed by [`EmbedProviders`].
pub struct LlmEmbedOffer<P> {
    entry: CatalogEntry,
    providers: P,
}

impl<P> LlmEmbedOffer<P> {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new(providers: P) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("llm.embed", "0.1.0")?,
            providers,
        })
    }
}

impl<P: EmbedProviders + Send + Sync> Offer for LlmEmbedOffer<P> {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-llm.embed".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_embed(&self.providers, &req.args).await {
            Ok(result) => InvokeResp::ok(invoke_id, result),
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
struct EmbedArgs {
    inputs: Vec<String>,
    #[serde(default)]
    model: Option<String>,
    /// Resolved provider id (`ollama`, `openai`, `echo`). Defaults to `ollama`
    /// (echo routers ignore the name).
    #[serde(default)]
    provider: Option<String>,
}

async fn run_embed<P: EmbedProviders>(
    providers: &P,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: EmbedArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("embed args: {e}")))?;
    if parsed.inputs.is_empty() {
        return Err((ErrorCode::SchemaInvalid, "inputs must be non-empty".into()));
    }
    let model = parsed.model.unwrap_or_else(|| "default".into());
    let provider = parsed.provider.unwrap_or_else(|| "ollama".into());
    let resp = providers
        .embed(
            &provider,
            EmbedRequest {
                model: model.clone(),
                inputs: parsed.inputs,
            },
        )
        .await
        .map_err(|e: ProviderError| (e.to_error_code(), e.to_string()))?;
    Ok(json!({
        "provider": provider,
        "model": resp.model.unwrap_or(model),
        "vectors": resp.vectors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EchoChatProvider, FakeEmbedProviders};
    use types::InvokeId;

    #[tokio::test]
    async fn echo_embed_offer() {
        let offer = LlmEmbedOffer::new(EchoChatProvider).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "inputs": ["xy"] }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["vectors"][0][0], 2.0);
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn named_provider_reaches_router() {
        let fake = FakeEmbedProviders::default();
        let offer = LlmEmbedOffer::new(fake.clone()).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "inputs": ["z"], "provider": "openai" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["provider"], "openai");
                assert_eq!(fake.last_provider().as_deref(), Some("openai"));
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
