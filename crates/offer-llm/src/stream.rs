//! Streaming chat collect (`sak137` / `refactor:offer-llm-stream`).

use provider_core::{ChatChunk, ChatRequest, ChatResponse, ProviderError};

use crate::ChatProviders;

/// Collect a provider stream into a full [`ChatResponse`] plus raw chunks.
///
/// # Errors
/// Provider errors from [`ChatProviders::chat_stream`].
pub async fn collect_chat_stream<P: ChatProviders>(
    providers: &P,
    provider: &str,
    req: ChatRequest,
) -> Result<(ChatResponse, Vec<ChatChunk>), ProviderError> {
    let chunks = providers.chat_stream(provider, req.clone()).await?;
    let mut text = String::new();
    for c in &chunks {
        text.push_str(&c.delta);
    }
    Ok((
        ChatResponse {
            content: text,
            usage: provider_core::TokenUsage {
                prompt_tokens: 1,
                completion_tokens: u64::try_from(chunks.len()).unwrap_or(1),
            },
            model: Some(req.model),
        },
        chunks,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EchoChatProvider;
    use provider_core::{ChatMessage, ChatRole};

    #[tokio::test]
    async fn echo_stream_collects() {
        let (resp, raw) = collect_chat_stream(
            &EchoChatProvider,
            "echo",
            ChatRequest {
                model: "m".into(),
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "hi there".into(),
                }],
                max_tokens: None,
                temperature: None,
                prompt_cache_key: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.content, "echo:hi there");
        assert!(!raw.is_empty());
        assert!(raw.last().unwrap().done);
    }
}
