//! Named-provider embed dispatch (`sak571-a`).

use provider_core::{EmbedRequest, EmbedResponse, LlmProvider, ProviderError};

/// Looks up a concrete embed backend by resolved provider id (`ollama`, `openai`, `echo`).
pub trait EmbedProviders: Send + Sync {
    /// Embed `req` on the named provider.
    fn embed(
        &self,
        provider: &str,
        req: EmbedRequest,
    ) -> impl std::future::Future<Output = Result<EmbedResponse, ProviderError>> + Send;
}

impl<P: LlmProvider + Sync> EmbedProviders for P {
    async fn embed(
        &self,
        _provider: &str,
        req: EmbedRequest,
    ) -> Result<EmbedResponse, ProviderError> {
        LlmProvider::embed(self, req).await
    }
}

/// Test double that records the last provider name and delegates to [`super::EchoChatProvider`].
#[derive(Clone, Debug, Default)]
pub struct FakeEmbedProviders {
    last_provider: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl FakeEmbedProviders {
    #[must_use]
    pub fn last_provider(&self) -> Option<String> {
        self.last_provider.lock().ok().and_then(|g| g.clone())
    }
}

impl EmbedProviders for FakeEmbedProviders {
    async fn embed(
        &self,
        provider: &str,
        req: EmbedRequest,
    ) -> Result<EmbedResponse, ProviderError> {
        if let Ok(mut g) = self.last_provider.lock() {
            *g = Some(provider.to_owned());
        }
        LlmProvider::embed(&crate::EchoChatProvider, req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_records_provider_and_echoes_vector() {
        let fake = FakeEmbedProviders::default();
        let resp = fake
            .embed(
                "ollama",
                EmbedRequest {
                    model: "nomic".into(),
                    inputs: vec!["hi".into()],
                },
            )
            .await
            .expect("embed");
        assert_eq!(fake.last_provider().as_deref(), Some("ollama"));
        assert_eq!(resp.vectors.len(), 1);
        assert!(!resp.vectors[0].is_empty());
    }

    #[tokio::test]
    async fn llm_provider_blanket_ignores_name() {
        let resp = EmbedProviders::embed(
            &crate::EchoChatProvider,
            "openai",
            EmbedRequest {
                model: "x".into(),
                inputs: vec!["ab".into()],
            },
        )
        .await
        .expect("embed");
        assert_eq!(resp.vectors.len(), 1);
    }
}
