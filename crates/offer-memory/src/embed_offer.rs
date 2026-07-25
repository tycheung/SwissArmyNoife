//! `memory.embed` — delegate embedding to an [`provider_core::LlmProvider`].

use control::{CatalogEntry, Offer};
use provider_core::{EmbedRequest, LlmProvider, ProviderError};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// First-party `memory.embed` offer (provider-backed).
pub struct MemoryEmbedOffer<P> {
    entry: CatalogEntry,
    provider: P,
}

impl<P> MemoryEmbedOffer<P> {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when offer id is empty.
    pub fn new(provider: P) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("memory.embed", "0.1.0")?,
            provider,
        })
    }
}

impl<P: LlmProvider + Send + Sync> Offer for MemoryEmbedOffer<P> {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-memory.embed".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_embed(&self.provider, &req.args).await {
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
}

async fn run_embed<P: LlmProvider>(
    provider: &P,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: EmbedArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("embed args: {e}")))?;
    if parsed.inputs.is_empty() {
        return Err((ErrorCode::SchemaInvalid, "inputs must be non-empty".into()));
    }
    let model = parsed.model.unwrap_or_else(|| "echo-embed".into());
    let resp = provider
        .embed(EmbedRequest {
            model: model.clone(),
            inputs: parsed.inputs,
        })
        .await
        .map_err(|e| map_provider(&e))?;
    Ok(json!({
        "model": resp.model.unwrap_or(model),
        "vectors": resp.vectors,
    }))
}

fn map_provider(e: &ProviderError) -> (ErrorCode, String) {
    (e.to_error_code(), e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_core::{EmbedRequest, EmbedResponse};
    use types::InvokeId;

    struct EchoEmbed;

    impl LlmProvider for EchoEmbed {
        async fn chat(
            &self,
            _req: provider_core::ChatRequest,
        ) -> Result<provider_core::ChatResponse, ProviderError> {
            Err(ProviderError::SchemaInvalid("chat unused".into()))
        }

        async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
            let vectors = req
                .inputs
                .iter()
                .map(|s| vec![f32::from(u16::try_from(s.len()).unwrap_or(u16::MAX)), 1.0])
                .collect();
            Ok(EmbedResponse {
                vectors,
                model: Some(req.model),
            })
        }
    }

    #[tokio::test]
    async fn embed_roundtrip() {
        let offer = MemoryEmbedOffer::new(EchoEmbed).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "inputs": ["ab", "c"] }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["vectors"].as_array().unwrap().len(), 2);
            }
            other @ InvokeResp::Error { .. } => panic!("expected ok, got {other:?}"),
        }
    }
}
