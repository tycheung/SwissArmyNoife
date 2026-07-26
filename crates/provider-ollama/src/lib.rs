//! Ollama HTTP provider (`/api/chat`, `/api/embed`).

use provider_core::{
    ChatRequest, ChatResponse, ChatRole, EmbedRequest, EmbedResponse, LlmProvider, ProviderError,
    TokenUsage,
};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE: &str = "http://127.0.0.1:11434";

/// Ollama OpenAPI-shaped client implementing [`LlmProvider`].
#[derive(Clone, Debug)]
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a client for `base_url` (no trailing slash).
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            client: reqwest::Client::new(),
        }
    }

    /// Default local daemon (`http://127.0.0.1:11434`).
    #[must_use]
    pub fn localhost() -> Self {
        Self::new(DEFAULT_BASE)
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// List local model names (`GET /api/tags`).
    ///
    /// # Errors
    /// Transport / HTTP / JSON failures.
    pub async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "ollama tags HTTP {status}: {text}"
            )));
        }
        let parsed: OllamaTagsResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::SchemaInvalid(e.to_string()))?;
        Ok(parsed.models.into_iter().map(|m| m.name).collect())
    }

    /// Pull a model (`POST /api/pull`, non-streaming).
    ///
    /// # Errors
    /// Transport / HTTP failures.
    pub async fn pull_model(&self, name: &str) -> Result<(), ProviderError> {
        if name.is_empty() {
            return Err(ProviderError::SchemaInvalid("model name empty".into()));
        }
        let url = format!("{}/api/pull", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&OllamaPullRequest {
                name: name.to_owned(),
                stream: false,
            })
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "ollama pull HTTP {status}: {text}"
            )));
        }
        Ok(())
    }

    /// Delete a local model (`DELETE /api/delete`).
    ///
    /// # Errors
    /// Transport / HTTP failures.
    pub async fn delete_model(&self, name: &str) -> Result<(), ProviderError> {
        if name.is_empty() {
            return Err(ProviderError::SchemaInvalid("model name empty".into()));
        }
        let url = format!("{}/api/delete", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .json(&OllamaDeleteRequest {
                name: name.to_owned(),
            })
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "ollama delete HTTP {status}: {text}"
            )));
        }
        Ok(())
    }
}

impl LlmProvider for OllamaProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let body = OllamaChatRequest {
            model: req.model.clone(),
            messages: req
                .messages
                .iter()
                .map(|m| OllamaMessage {
                    role: role_str(&m.role),
                    content: m.content.clone(),
                })
                .collect(),
            stream: false,
            options: chat_options(&req),
        };
        let url = format!("{}/api/chat", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "ollama chat HTTP {status}: {text}"
            )));
        }
        let parsed: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::SchemaInvalid(e.to_string()))?;
        Ok(ChatResponse {
            content: parsed.message.content,
            usage: TokenUsage {
                prompt_tokens: u64::from(parsed.prompt_eval_count.unwrap_or(0)),
                completion_tokens: u64::from(parsed.eval_count.unwrap_or(0)),
            },
            model: Some(parsed.model.unwrap_or(req.model)),
        })
    }

    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
        if req.inputs.is_empty() {
            return Err(ProviderError::SchemaInvalid("inputs empty".into()));
        }
        let body = OllamaEmbedRequest {
            model: req.model.clone(),
            input: req.inputs.clone(),
        };
        let url = format!("{}/api/embed", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unreachable(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Unreachable(format!(
                "ollama embed HTTP {status}: {text}"
            )));
        }
        let parsed: OllamaEmbedResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::SchemaInvalid(e.to_string()))?;
        Ok(EmbedResponse {
            vectors: parsed.embeddings,
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

fn chat_options(req: &ChatRequest) -> Option<OllamaOptions> {
    if req.temperature.is_none() && req.max_tokens.is_none() {
        return None;
    }
    Some(OllamaOptions {
        temperature: req.temperature,
        num_predict: req.max_tokens,
    })
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    model: Option<String>,
    message: OllamaChatMessage,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaChatMessage {
    content: String,
}

#[derive(Serialize)]
struct OllamaEmbedRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    model: Option<String>,
    embeddings: Vec<Vec<f32>>,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagModel>,
}

#[derive(Deserialize)]
struct OllamaTagModel {
    name: String,
}

#[derive(Serialize)]
struct OllamaPullRequest {
    name: String,
    stream: bool,
}

#[derive(Serialize)]
struct OllamaDeleteRequest {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_core::{ChatMessage, LlmProvider};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_maps_ollama_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "llama3.2",
                "message": { "role": "assistant", "content": "hello" },
                "prompt_eval_count": 3,
                "eval_count": 2
            })))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
        let resp = provider
            .chat(ChatRequest {
                model: "llama3.2".into(),
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
        assert_eq!(resp.usage.prompt_tokens, 3);
        assert_eq!(resp.usage.completion_tokens, 2);
    }

    #[tokio::test]
    async fn embed_maps_ollama_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "nomic-embed-text",
                "embeddings": [[0.1, 0.2], [0.3, 0.4]]
            })))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
        let resp = provider
            .embed(EmbedRequest {
                model: "nomic-embed-text".into(),
                inputs: vec!["a".into(), "b".into()],
            })
            .await
            .expect("embed");
        assert_eq!(resp.vectors.len(), 2);
        assert!((resp.vectors[0][0] - 0.1).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn list_models_maps_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{ "name": "tiny" }, { "name": "big" }]
            })))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
        let models = provider.list_models().await.expect("list");
        assert_eq!(models, vec!["tiny".to_string(), "big".to_string()]);
    }

    #[tokio::test]
    async fn http_error_is_unreachable() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
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
