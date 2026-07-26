//! OpenAI-compatible HTTP provider (`/v1/chat/completions`, `/v1/embeddings`).

use provider_core::{
    ChatRequest, ChatResponse, ChatRole, EmbedRequest, EmbedResponse, LlmProvider, ProviderError,
    TokenUsage,
};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE: &str = "https://api.openai.com";

/// OpenAI-compatible client implementing [`LlmProvider`].
#[derive(Clone)]
pub struct OpenAiProvider {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .finish_non_exhaustive()
    }
}

impl OpenAiProvider {
    /// Create a client for `base_url` (no trailing slash). Optional Bearer `api_key`.
    #[must_use]
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Official `OpenAI` API base (`https://api.openai.com`).
    #[must_use]
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::new(DEFAULT_BASE, Some(api_key.into()))
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.bearer_auth(key),
            None => req,
        }
    }
}

impl LlmProvider for OpenAiProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let body = OpenAiChatRequest {
            model: req.model.clone(),
            messages: req
                .messages
                .iter()
                .map(|m| OpenAiMessage {
                    role: role_str(&m.role),
                    content: m.content.clone(),
                })
                .collect(),
            max_tokens: req.max_tokens,
            temperature: req.temperature,
        };
        let url = format!("{}/v1/chat/completions", self.base_url);
        let builder = self.apply_auth(self.client.post(&url).json(&body));
        let resp = builder
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "openai chat HTTP {status}: {text}"
            )));
        }
        let parsed: OpenAiChatResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::SchemaInvalid(e.to_string()))?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::SchemaInvalid("no choices".into()))?;
        let usage = parsed.usage.unwrap_or_default();
        Ok(ChatResponse {
            content: choice.message.content.unwrap_or_default(),
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
            },
            model: Some(parsed.model.unwrap_or(req.model)),
        })
    }

    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
        if req.inputs.is_empty() {
            return Err(ProviderError::SchemaInvalid("inputs empty".into()));
        }
        let body = OpenAiEmbedRequest {
            model: req.model.clone(),
            input: req.inputs.clone(),
        };
        let url = format!("{}/v1/embeddings", self.base_url);
        let builder = self.apply_auth(self.client.post(&url).json(&body));
        let resp = builder
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "openai embed HTTP {status}: {text}"
            )));
        }
        let parsed: OpenAiEmbedResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::SchemaInvalid(e.to_string()))?;
        let mut indexed: Vec<_> = parsed.data.into_iter().collect();
        indexed.sort_by_key(|d| d.index);
        Ok(EmbedResponse {
            vectors: indexed.into_iter().map(|d| d.embedding).collect(),
            model: Some(parsed.model.unwrap_or(req.model)),
        })
    }
}

fn role_str(role: &ChatRole) -> &'static str {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
    }
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    model: Option<String>,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
}

#[derive(Default, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Serialize)]
struct OpenAiEmbedRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct OpenAiEmbedResponse {
    model: Option<String>,
    data: Vec<OpenAiEmbedData>,
}

#[derive(Deserialize)]
struct OpenAiEmbedData {
    index: u32,
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_core::{ChatMessage, LlmProvider};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_maps_openai_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "gpt-4o-mini",
                "choices": [{
                    "message": { "role": "assistant", "content": "hello" }
                }],
                "usage": { "prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7 }
            })))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new(server.uri(), Some("test-key".into()));
        let resp = provider
            .chat(ChatRequest {
                model: "gpt-4o-mini".into(),
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "hi".into(),
                }],
                max_tokens: Some(32),
                temperature: Some(0.1),
                prompt_cache_key: None,
            })
            .await
            .expect("chat");
        assert_eq!(resp.content, "hello");
        assert_eq!(resp.usage.prompt_tokens, 5);
        assert_eq!(resp.usage.completion_tokens, 2);
    }

    #[tokio::test]
    async fn embed_maps_openai_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "text-embedding-3-small",
                "data": [
                    { "index": 1, "embedding": [0.3, 0.4] },
                    { "index": 0, "embedding": [0.1, 0.2] }
                ]
            })))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new(server.uri(), None);
        let resp = provider
            .embed(EmbedRequest {
                model: "text-embedding-3-small".into(),
                inputs: vec!["a".into(), "b".into()],
            })
            .await
            .expect("embed");
        assert_eq!(resp.vectors.len(), 2);
        assert!((resp.vectors[0][0] - 0.1).abs() < f32::EPSILON);
        assert!((resp.vectors[1][0] - 0.3).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn http_error_is_unreachable() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let provider = OpenAiProvider::new(server.uri(), None);
        let err = provider
            .chat(ChatRequest {
                model: "x".into(),
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "hi".into(),
                }],
                max_tokens: None,
                temperature: None,
                prompt_cache_key: None,
            })
            .await
            .expect_err("fail");
        match err {
            ProviderError::Unreachable(msg) => assert!(msg.contains("500")),
            other => panic!("unexpected {other:?}"),
        }
    }
}
