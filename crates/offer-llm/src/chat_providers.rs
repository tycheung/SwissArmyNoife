//! Multi-provider chat dispatch after resolve picks a provider name.

use provider_core::{ChatChunk, ChatRequest, ChatResponse, LlmProvider, ProviderError};

use crate::echo_chunks::echo_chunks;

/// Looks up a concrete [`LlmProvider`] by resolved provider id (`ollama`, `openai`, …).
///
/// Single-provider adapters implement this by ignoring `provider` and forwarding to
/// [`LlmProvider::chat`].
pub trait ChatProviders: Send + Sync {
    /// Run chat on the named provider.
    fn chat(
        &self,
        provider: &str,
        req: ChatRequest,
    ) -> impl std::future::Future<Output = Result<ChatResponse, ProviderError>> + Send;

    /// Stream chat deltas (`sak137-a`).
    fn chat_stream(
        &self,
        provider: &str,
        req: ChatRequest,
    ) -> impl std::future::Future<Output = Result<Vec<ChatChunk>, ProviderError>> + Send;
}

impl<P: LlmProvider + Sync> ChatProviders for P {
    async fn chat(&self, _provider: &str, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        LlmProvider::chat(self, req).await
    }

    async fn chat_stream(
        &self,
        _provider: &str,
        req: ChatRequest,
    ) -> Result<Vec<ChatChunk>, ProviderError> {
        LlmProvider::chat_stream(self, req).await
    }
}

/// Deterministic in-process provider for CI / `LLM_BACKEND=echo`.
#[derive(Clone, Debug, Default)]
pub struct EchoChatProvider;

impl LlmProvider for EchoChatProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let last = req
            .messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        Ok(ChatResponse {
            content: format!("echo:{last}"),
            usage: provider_core::TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
            },
            model: Some(req.model),
        })
    }

    async fn chat_stream(&self, req: ChatRequest) -> Result<Vec<ChatChunk>, ProviderError> {
        let resp = LlmProvider::chat(self, req).await?;
        Ok(echo_chunks(&resp.content))
    }

    async fn embed(
        &self,
        req: provider_core::EmbedRequest,
    ) -> Result<provider_core::EmbedResponse, ProviderError> {
        if req.inputs.is_empty() {
            return Err(ProviderError::SchemaInvalid("inputs empty".into()));
        }
        let vectors = req
            .inputs
            .iter()
            .map(|s| vec![f32::from(u16::try_from(s.len()).unwrap_or(u16::MAX)), 1.0])
            .collect();
        Ok(provider_core::EmbedResponse {
            vectors,
            model: Some(req.model),
        })
    }
}
