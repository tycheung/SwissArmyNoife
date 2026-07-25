//! `llm.chat` offer: resolve → provider chat → JSON result (no secrets).

use control::{CatalogEntry, Offer};
use provider_core::{ChatMessage, ChatRequest, ChatResponse, ChatRole, ProviderError};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::{
    collect_chat_stream, resolve, BindingSource, ChatProviders, ConnectionRef, ResolveError,
    ResolveHint,
};

/// First-party `llm.chat` offer backed by [`ChatProviders`].
pub struct LlmChatOffer<P> {
    entry: CatalogEntry,
    providers: P,
    connections: Vec<ConnectionRef>,
}

impl<P> LlmChatOffer<P> {
    /// Build the offer with a metadata-only connection catalog for resolve.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the offer id is empty.
    pub fn new(providers: P, connections: Vec<ConnectionRef>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("llm.chat", "0.1.0")?,
            providers,
            connections,
        })
    }

    #[must_use]
    pub fn connections(&self) -> &[ConnectionRef] {
        &self.connections
    }
}

impl<P: ChatProviders + Send + Sync> Offer for LlmChatOffer<P> {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-llm.chat".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_chat(&self.providers, &self.connections, &req.args).await {
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
struct ChatArgs {
    messages: Vec<WireMessage>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    connection_id: Option<String>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
    /// When true, collect provider stream into `text` + `chunks` (`sak137-c`).
    #[serde(default)]
    stream: bool,
    /// Optional provider prompt-cache key (passthrough; backends may ignore).
    #[serde(default)]
    prompt_cache_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireMessage {
    role: String,
    content: String,
}

async fn run_chat<P: ChatProviders>(
    providers: &P,
    connections: &[ConnectionRef],
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: ChatArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("chat args: {e}")))?;
    if parsed.messages.is_empty() {
        return Err((
            ErrorCode::SchemaInvalid,
            "messages must be non-empty".into(),
        ));
    }
    let messages = parsed
        .messages
        .iter()
        .map(wire_message)
        .collect::<Result<Vec<_>, _>>()?;
    let resolved = resolve(
        &ResolveHint {
            connection_id: parsed.connection_id.clone(),
            provider: parsed.provider.clone(),
            model: parsed.model.clone(),
        },
        connections,
    )
    .map_err(|e| map_resolve(&e))?;
    let req = ChatRequest {
        model: resolved.model.clone(),
        messages,
        max_tokens: parsed.max_tokens,
        temperature: parsed.temperature,
        prompt_cache_key: parsed.prompt_cache_key.clone(),
    };
    if parsed.stream {
        let (chat, chunks) = collect_chat_stream(providers, &resolved.provider, req)
            .await
            .map_err(|e| map_provider(&e))?;
        let mut out = result_json(
            &resolved.provider,
            resolved.binding_source,
            resolved.connection_id.as_ref(),
            &chat,
        );
        out["streamed"] = json!(true);
        out["chunks"] =
            serde_json::to_value(&chunks).map_err(|e| (ErrorCode::SchemaInvalid, e.to_string()))?;
        return Ok(out);
    }
    let chat = providers
        .chat(&resolved.provider, req)
        .await
        .map_err(|e| map_provider(&e))?;
    Ok(result_json(
        &resolved.provider,
        resolved.binding_source,
        resolved.connection_id.as_ref(),
        &chat,
    ))
}

fn result_json(
    provider: &str,
    binding_source: BindingSource,
    connection_id: Option<&String>,
    chat: &ChatResponse,
) -> Value {
    json!({
        "text": chat.content,
        "model": chat.model,
        "provider": provider,
        "connection_id": connection_id,
        "binding_source": binding_source,
        "usage": {
            "prompt_tokens": chat.usage.prompt_tokens,
            "completion_tokens": chat.usage.completion_tokens,
            "total_tokens": chat.usage.total(),
        }
    })
}

fn wire_message(m: &WireMessage) -> Result<ChatMessage, (ErrorCode, String)> {
    let role = match m.role.as_str() {
        "system" => ChatRole::System,
        "user" => ChatRole::User,
        "assistant" => ChatRole::Assistant,
        "tool" => ChatRole::Tool,
        other => {
            return Err((
                ErrorCode::SchemaInvalid,
                format!("unknown message role: {other}"),
            ));
        }
    };
    Ok(ChatMessage {
        role,
        content: m.content.clone(),
    })
}

fn map_resolve(err: &ResolveError) -> (ErrorCode, String) {
    (err.to_error_code(), err.to_string())
}

fn map_provider(err: &ProviderError) -> (ErrorCode, String) {
    (err.to_error_code(), err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EchoChatProvider;
    use serde_json::json;
    use types::InvokeId;

    #[tokio::test]
    async fn prompt_cache_key_roundtrips_into_chat_request() {
        let offer = LlmChatOffer::new(EchoChatProvider, vec![]).expect("offer");
        let cache_key = "session-abc-123";
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "messages": [{"role": "user", "content": "ping"}],
                    "model": "fixture-model",
                    "prompt_cache_key": cache_key
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { .. } => {}
            InvokeResp::Error { code, message, .. } => {
                panic!("unexpected error {code}: {message}")
            }
        }
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "hi".into(),
            }],
            max_tokens: None,
            temperature: None,
            prompt_cache_key: Some(cache_key.into()),
        };
        let back: ChatRequest =
            serde_json::from_value(serde_json::to_value(&req).expect("ser")).expect("de");
        assert_eq!(back.prompt_cache_key.as_deref(), Some(cache_key));
    }

    #[tokio::test]
    async fn invoke_chat_returns_text_and_binding_source() {
        let offer = LlmChatOffer::new(EchoChatProvider, vec![]).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "messages": [{"role": "user", "content": "ping"}],
                    "model": "fixture-model"
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["text"], "echo:ping");
                assert_eq!(result["provider"], "ollama");
                assert_eq!(result["binding_source"], "local_ollama");
                assert_eq!(result["usage"]["total_tokens"], 2);
                assert!(result.get("api_key").is_none());
            }
            InvokeResp::Error { code, message, .. } => {
                panic!("unexpected error {code}: {message}")
            }
        }
    }

    #[tokio::test]
    async fn stream_true_returns_chunks() {
        let offer = LlmChatOffer::new(EchoChatProvider, vec![]).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "messages": [{"role": "user", "content": "ping"}],
                    "model": "fixture-model",
                    "stream": true
                }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["text"], "echo:ping");
                assert_eq!(result["streamed"], true);
                assert!(result["chunks"].as_array().unwrap().len() >= 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_messages_is_schema_invalid() {
        let offer = LlmChatOffer::new(EchoChatProvider, vec![]).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({"messages": []}),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error {
                code: ErrorCode::SchemaInvalid,
                ..
            } => {}
            other => panic!("expected SchemaInvalid, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_vault_connection_surfaces_vault_missing() {
        let offer = LlmChatOffer::new(EchoChatProvider, vec![]).expect("offer");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({
                    "messages": [{"role": "user", "content": "hi"}],
                    "provider": "openai"
                }),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Error {
                code: ErrorCode::VaultMissing,
                ..
            } => {}
            other => panic!("expected VaultMissing, got {other:?}"),
        }
    }
}
