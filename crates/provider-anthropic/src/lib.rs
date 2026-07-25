//! Anthropic Messages API provider (`/v1/messages`). Embeddings are unsupported.

use provider_core::{
    ChatRequest, ChatResponse, ChatRole, EmbedRequest, EmbedResponse, LlmProvider, ProviderError,
    TokenUsage,
};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// Anthropic Messages client implementing [`LlmProvider`].
#[derive(Clone)]
pub struct AnthropicProvider {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl std::fmt::Debug for AnthropicProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicProvider")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .finish_non_exhaustive()
    }
}

impl AnthropicProvider {
    /// Create a client for `base_url` (no trailing slash). Optional `x-api-key`.
    #[must_use]
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Official Anthropic API base (`https://api.anthropic.com`).
    #[must_use]
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self::new(DEFAULT_BASE, Some(api_key.into()))
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn apply_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = req.header("anthropic-version", ANTHROPIC_VERSION);
        if let Some(key) = &self.api_key {
            req = req.header("x-api-key", key);
        }
        req
    }
}

impl LlmProvider for AnthropicProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let (system, messages) = split_system(&req)?;
        let body = AnthropicMessagesRequest {
            model: req.model.clone(),
            max_tokens: req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system,
            messages,
            temperature: req.temperature,
        };
        let url = format!("{}/v1/messages", self.base_url);
        let builder = self.apply_headers(self.client.post(&url).json(&body));
        let resp = builder
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "anthropic chat HTTP {status}: {text}"
            )));
        }
        let parsed: AnthropicMessagesResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::SchemaInvalid(e.to_string()))?;
        let content = join_text_blocks(&parsed.content);
        let usage = parsed.usage.unwrap_or_default();
        Ok(ChatResponse {
            content,
            usage: TokenUsage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
            },
            model: Some(parsed.model.unwrap_or(req.model)),
        })
    }

    async fn embed(&self, _req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
        Err(ProviderError::SchemaInvalid(
            "anthropic does not expose embeddings".into(),
        ))
    }
}

fn split_system(
    req: &ChatRequest,
) -> Result<(Option<String>, Vec<AnthropicMessage>), ProviderError> {
    let mut system_parts: Vec<String> = Vec::new();
    let mut messages: Vec<AnthropicMessage> = Vec::new();
    for m in &req.messages {
        match m.role {
            ChatRole::System => system_parts.push(m.content.clone()),
            ChatRole::User => messages.push(AnthropicMessage {
                role: "user",
                content: m.content.clone(),
            }),
            ChatRole::Assistant => messages.push(AnthropicMessage {
                role: "assistant",
                content: m.content.clone(),
            }),
            ChatRole::Tool => {
                return Err(ProviderError::SchemaInvalid(
                    "anthropic adapter does not map tool roles yet".into(),
                ));
            }
        }
    }
    if messages.is_empty() {
        return Err(ProviderError::SchemaInvalid(
            "anthropic requires at least one non-system message".into(),
        ));
    }
    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n"))
    };
    Ok((system, messages))
}

fn join_text_blocks(blocks: &[AnthropicContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|b| match b {
            AnthropicContentBlock::Text { text } => Some(text.as_str()),
            AnthropicContentBlock::Other => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Serialize)]
struct AnthropicMessagesRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicMessagesResponse {
    model: Option<String>,
    content: Vec<AnthropicContentBlock>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Default, Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_core::{ChatMessage, LlmProvider};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_maps_anthropic_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .and(header("anthropic-version", ANTHROPIC_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "claude-3-5-haiku-latest",
                "content": [
                    { "type": "text", "text": "hel" },
                    { "type": "text", "text": "lo" }
                ],
                "usage": { "input_tokens": 4, "output_tokens": 2 }
            })))
            .mount(&server)
            .await;

        let provider = AnthropicProvider::new(server.uri(), Some("test-key".into()));
        let resp = provider
            .chat(ChatRequest {
                model: "claude-3-5-haiku-latest".into(),
                messages: vec![
                    ChatMessage {
                        role: ChatRole::System,
                        content: "be brief".into(),
                    },
                    ChatMessage {
                        role: ChatRole::User,
                        content: "hi".into(),
                    },
                ],
                max_tokens: Some(64),
                temperature: Some(0.2),
            })
            .await
            .expect("chat");
        assert_eq!(resp.content, "hello");
        assert_eq!(resp.usage.prompt_tokens, 4);
        assert_eq!(resp.usage.completion_tokens, 2);
    }

    #[tokio::test]
    async fn embed_is_schema_invalid() {
        let provider = AnthropicProvider::new("http://127.0.0.1:9", None);
        let err = provider
            .embed(EmbedRequest {
                model: "x".into(),
                inputs: vec!["a".into()],
            })
            .await
            .expect_err("no embed");
        match err {
            ProviderError::SchemaInvalid(msg) => assert!(msg.contains("embeddings")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_error_is_unreachable() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let provider = AnthropicProvider::new(server.uri(), None);
        let err = provider
            .chat(ChatRequest {
                model: "x".into(),
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "hi".into(),
                }],
                max_tokens: None,
                temperature: None,
            })
            .await
            .expect_err("fail");
        match err {
            ProviderError::Unreachable(msg) => assert!(msg.contains("500")),
            other => panic!("unexpected {other:?}"),
        }
    }
}
