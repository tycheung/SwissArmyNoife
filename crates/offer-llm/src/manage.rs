//! `llm.ollama.manage` — list / pull / delete local Ollama models (`sak139`).

use control::{CatalogEntry, Offer};
use provider_core::ProviderError;
use provider_ollama::OllamaProvider;
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// Model-hub style manage offer backed by [`OllamaProvider`].
pub struct LlmOllamaManageOffer {
    entry: CatalogEntry,
    provider: OllamaProvider,
}

impl LlmOllamaManageOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new(provider: OllamaProvider) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("llm.ollama.manage", "0.1.0")?,
            provider,
        })
    }

    /// Localhost daemon client.
    ///
    /// # Errors
    /// Invalid catalog id.
    pub fn localhost() -> Result<Self, ErrorCode> {
        Self::new(OllamaProvider::localhost())
    }
}

impl Offer for LlmOllamaManageOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-llm.ollama.manage".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run(&self.provider, &req.args).await {
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
struct ManageArgs {
    action: String,
    #[serde(default)]
    model: Option<String>,
}

async fn run(provider: &OllamaProvider, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: ManageArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("manage args: {e}")))?;
    match parsed.action.as_str() {
        "list" => {
            let models = provider.list_models().await.map_err(|e| map_err(&e))?;
            Ok(json!({ "models": models }))
        }
        "pull" => {
            let name = parsed
                .model
                .filter(|s| !s.is_empty())
                .ok_or_else(|| (ErrorCode::SchemaInvalid, "pull requires model".into()))?;
            provider.pull_model(&name).await.map_err(|e| map_err(&e))?;
            Ok(json!({ "pulled": name }))
        }
        "delete" => {
            let name = parsed
                .model
                .filter(|s| !s.is_empty())
                .ok_or_else(|| (ErrorCode::SchemaInvalid, "delete requires model".into()))?;
            provider
                .delete_model(&name)
                .await
                .map_err(|e| map_err(&e))?;
            Ok(json!({ "deleted": name }))
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("action must be list|pull|delete, got {other}"),
        )),
    }
}

fn map_err(e: &ProviderError) -> (ErrorCode, String) {
    (e.to_error_code(), e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_maps_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{ "name": "llama3.2:latest" }]
            })))
            .mount(&server)
            .await;

        let offer = LlmOllamaManageOffer::new(OllamaProvider::new(server.uri())).unwrap();
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::from_uuid(Uuid::nil()),
                args: json!({ "action": "list" }),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["models"][0], "llama3.2:latest");
            }
            other @ InvokeResp::Error { .. } => panic!("unexpected {other:?}"),
        }
    }
}
