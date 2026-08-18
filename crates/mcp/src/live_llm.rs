//! MCP LLM provider router (`LLM_BACKEND` echo vs live Ollama/OpenAI/Anthropic).

use offer_llm::{ChatProviders, EchoChatProvider, EmbedProviders};
use provider_anthropic::AnthropicProvider;
use provider_core::{
    ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmProvider, ProviderError,
};
use provider_ollama::OllamaProvider;
use provider_openai::OpenAiProvider;

/// `LLM_BACKEND`: `ollama` (default) or `echo` (CI / no daemon).
pub const LLM_BACKEND: &str = "LLM_BACKEND";

/// Routes resolved provider names to concrete HTTP clients (or echo).
pub struct McpLlmRouter {
    mode: LlmMode,
}

enum LlmMode {
    Echo(EchoChatProvider),
    Live {
        ollama: OllamaProvider,
        openai: Option<OpenAiProvider>,
        anthropic: Option<AnthropicProvider>,
    },
}

impl McpLlmRouter {
    #[must_use]
    pub fn from_env() -> Self {
        let mode = if std::env::var(LLM_BACKEND)
            .unwrap_or_default()
            .eq_ignore_ascii_case("echo")
        {
            tracing::info!("llm backend=echo (deterministic)");
            LlmMode::Echo(EchoChatProvider)
        } else {
            let openai = std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|s| !s.is_empty())
                .map(OpenAiProvider::openai);
            let anthropic = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|s| !s.is_empty())
                .map(AnthropicProvider::anthropic);
            tracing::info!(
                openai = openai.is_some(),
                anthropic = anthropic.is_some(),
                "llm backend=ollama (+ optional cloud keys)"
            );
            LlmMode::Live {
                ollama: OllamaProvider::localhost(),
                openai,
                anthropic,
            }
        };
        Self { mode }
    }

    fn default_provider(&self) -> &'static str {
        match &self.mode {
            LlmMode::Echo(_) => "echo",
            LlmMode::Live { .. } => "ollama",
        }
    }
}

/// [`LlmProvider`] view of [`McpLlmRouter`] for `memory.embed` (`sak571-c`).
pub struct RouterAsLlm(pub McpLlmRouter);

impl LlmProvider for RouterAsLlm {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        ChatProviders::chat(&self.0, self.0.default_provider(), req).await
    }

    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse, ProviderError> {
        EmbedProviders::embed(&self.0, self.0.default_provider(), req).await
    }
}

impl ChatProviders for McpLlmRouter {
    async fn chat(&self, provider: &str, req: ChatRequest) -> Result<ChatResponse, ProviderError> {
        match &self.mode {
            LlmMode::Echo(echo) => ChatProviders::chat(echo, provider, req).await,
            LlmMode::Live {
                ollama,
                openai,
                anthropic,
            } => match provider {
                "ollama" => ChatProviders::chat(ollama, provider, req).await,
                "openai" => match openai {
                    Some(p) => ChatProviders::chat(p, provider, req).await,
                    None => Err(ProviderError::Unreachable(
                        "openai: set OPENAI_API_KEY".into(),
                    )),
                },
                "anthropic" => match anthropic {
                    Some(p) => ChatProviders::chat(p, provider, req).await,
                    None => Err(ProviderError::Unreachable(
                        "anthropic: set ANTHROPIC_API_KEY".into(),
                    )),
                },
                other => Err(ProviderError::SchemaInvalid(format!(
                    "unsupported provider: {other}"
                ))),
            },
        }
    }

    async fn chat_stream(
        &self,
        provider: &str,
        req: ChatRequest,
    ) -> Result<Vec<provider_core::ChatChunk>, ProviderError> {
        match &self.mode {
            LlmMode::Echo(echo) => ChatProviders::chat_stream(echo, provider, req).await,
            LlmMode::Live {
                ollama,
                openai,
                anthropic,
            } => match provider {
                "ollama" => ChatProviders::chat_stream(ollama, provider, req).await,
                "openai" => match openai {
                    Some(p) => ChatProviders::chat_stream(p, provider, req).await,
                    None => Err(ProviderError::Unreachable(
                        "openai: set OPENAI_API_KEY".into(),
                    )),
                },
                "anthropic" => match anthropic {
                    Some(p) => ChatProviders::chat_stream(p, provider, req).await,
                    None => Err(ProviderError::Unreachable(
                        "anthropic: set ANTHROPIC_API_KEY".into(),
                    )),
                },
                other => Err(ProviderError::SchemaInvalid(format!(
                    "unsupported provider: {other}"
                ))),
            },
        }
    }
}

impl EmbedProviders for McpLlmRouter {
    async fn embed(
        &self,
        provider: &str,
        req: EmbedRequest,
    ) -> Result<EmbedResponse, ProviderError> {
        match &self.mode {
            LlmMode::Echo(echo) => EmbedProviders::embed(echo, provider, req).await,
            LlmMode::Live {
                ollama,
                openai,
                anthropic,
            } => match provider {
                "ollama" => EmbedProviders::embed(ollama, provider, req).await,
                "openai" => match openai {
                    Some(p) => EmbedProviders::embed(p, provider, req).await,
                    None => Err(ProviderError::Unreachable(
                        "openai: set OPENAI_API_KEY".into(),
                    )),
                },
                "anthropic" => match anthropic {
                    Some(p) => EmbedProviders::embed(p, provider, req).await,
                    None => Err(ProviderError::Unreachable(
                        "anthropic: set ANTHROPIC_API_KEY".into(),
                    )),
                },
                other => Err(ProviderError::SchemaInvalid(format!(
                    "unsupported provider: {other}"
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_backend_embed_is_ci_safe() {
        std::env::set_var(LLM_BACKEND, "echo");
        let router = McpLlmRouter::from_env();
        let resp = EmbedProviders::embed(
            &router,
            "ollama",
            EmbedRequest {
                model: "nomic".into(),
                inputs: vec!["ab".into()],
            },
        )
        .await
        .expect("echo embed");
        assert!((resp.vectors[0][0] - 2.0).abs() < f32::EPSILON);
        std::env::remove_var(LLM_BACKEND);
    }

    #[tokio::test]
    async fn memory_embed_uses_router_echo_vectors() {
        std::env::set_var(LLM_BACKEND, "echo");
        let as_llm = RouterAsLlm(McpLlmRouter::from_env());
        let resp = LlmProvider::embed(
            &as_llm,
            EmbedRequest {
                model: "echo-embed".into(),
                inputs: vec!["abc".into()],
            },
        )
        .await
        .expect("memory embed path");
        assert!(!resp.vectors.is_empty());
        assert!(resp.vectors[0].iter().any(|x| *x != 0.0));
        std::env::remove_var(LLM_BACKEND);
    }
}
