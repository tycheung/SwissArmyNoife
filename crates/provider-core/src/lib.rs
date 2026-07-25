//! LLM provider protocol: chat + embed (no HTTP clients here).

mod chat;
mod embed;
mod error;

pub use chat::{ChatChunk, ChatMessage, ChatRequest, ChatResponse, ChatRole, TokenUsage};
pub use embed::{EmbedRequest, EmbedResponse};
pub use error::ProviderError;
pub use types::ErrorCode;

/// Async provider surface used by `llm.*` offers.
pub trait LlmProvider: Send + Sync {
    /// Chat / completions style generation.
    fn chat(
        &self,
        req: ChatRequest,
    ) -> impl std::future::Future<Output = Result<ChatResponse, ProviderError>> + Send;

    /// Streaming deltas; default is a single full-content chunk + done.
    fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> impl std::future::Future<Output = Result<Vec<ChatChunk>, ProviderError>> + Send {
        async move {
            let resp = self.chat(req).await?;
            Ok(vec![
                ChatChunk::delta(resp.content),
                ChatChunk::final_chunk(),
            ])
        }
    }

    /// Embedding vectors for one or more inputs.
    fn embed(
        &self,
        req: EmbedRequest,
    ) -> impl std::future::Future<Output = Result<EmbedResponse, ProviderError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoProvider;

    impl LlmProvider for EchoProvider {
        async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
            let last = req
                .messages
                .last()
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Ok(ChatResponse {
                content: format!("echo:{last}"),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                },
                model: Some(req.model),
            })
        }

        async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
            if req.inputs.is_empty() {
                return Err(ProviderError::SchemaInvalid("inputs empty".into()));
            }
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
    async fn echo_chat_and_embed() {
        let p = EchoProvider;
        let chat = p
            .chat(ChatRequest {
                model: "echo".into(),
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "hi".into(),
                }],
                max_tokens: None,
                temperature: None,
            })
            .await
            .expect("chat");
        assert_eq!(chat.content, "echo:hi");
        assert_eq!(chat.usage.total(), 2);

        let emb = p
            .embed(EmbedRequest {
                model: "echo".into(),
                inputs: vec!["ab".into(), "c".into()],
            })
            .await
            .expect("embed");
        assert_eq!(emb.vectors.len(), 2);
        assert!((emb.vectors[0][0] - 2.0).abs() < f32::EPSILON);
        assert!((emb.vectors[0][1] - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn embed_empty_is_schema_invalid() {
        let p = EchoProvider;
        let err = p
            .embed(EmbedRequest {
                model: "echo".into(),
                inputs: vec![],
            })
            .await
            .expect_err("empty");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }

    #[test]
    fn chat_req_roundtrip() {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![ChatMessage {
                role: ChatRole::System,
                content: "sys".into(),
            }],
            max_tokens: Some(16),
            temperature: Some(0.2),
            prompt_cache_key: Some("cache-key-1".into()),
        };
        let v = serde_json::to_value(&req).expect("ser");
        let back: ChatRequest = serde_json::from_value(v).expect("de");
        assert_eq!(back, req);
    }
}
