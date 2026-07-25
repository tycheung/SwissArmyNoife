//! `Offer` capability contract (catalog → provision → bind → invoke → unbind).

use serde_json::Value;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp, OfferId};

/// Static catalog metadata for an offer module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: OfferId,
    pub version: String,
}

impl CatalogEntry {
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when `id` is empty.
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Result<Self, ErrorCode> {
        Ok(Self {
            id: OfferId::new(id)?,
            version: version.into(),
        })
    }
}

/// Provider-facing offer surface. Control plane dispatches bind/invoke through this trait.
pub trait Offer: Send + Sync {
    fn catalog_entry(&self) -> &CatalogEntry;

    /// Allocate provider resources (model pull, sandbox image, index slot, …).
    fn provision(
        &self,
        params: Value,
    ) -> impl std::future::Future<Output = Result<String, ErrorCode>> + Send;

    /// Attach a binding id with frozen policy params (opaque JSON for now).
    fn bind(
        &self,
        binding_id: BindingId,
        params: Value,
    ) -> impl std::future::Future<Output = Result<(), ErrorCode>> + Send;

    /// Execute against a binding.
    fn invoke(&self, req: InvokeReq) -> impl std::future::Future<Output = InvokeResp> + Send;

    /// Release binding resources.
    fn unbind(
        &self,
        binding_id: BindingId,
    ) -> impl std::future::Future<Output = Result<(), ErrorCode>> + Send;

    /// Liveness / readiness probe.
    fn health(&self) -> impl std::future::Future<Output = Result<(), ErrorCode>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use types::InvokeId;

    struct EchoOffer {
        entry: CatalogEntry,
    }

    impl EchoOffer {
        fn new() -> Self {
            Self {
                entry: CatalogEntry::new("test.echo", "0.1.0").expect("valid"),
            }
        }
    }

    impl Offer for EchoOffer {
        fn catalog_entry(&self) -> &CatalogEntry {
            &self.entry
        }

        async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
            Ok("resource-echo-1".into())
        }

        async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
            Ok(())
        }

        async fn invoke(&self, req: InvokeReq) -> InvokeResp {
            let invoke_id = req.invoke_id.unwrap_or_default();
            InvokeResp::ok(invoke_id, req.args)
        }

        async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
            Ok(())
        }

        async fn health(&self) -> Result<(), ErrorCode> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn mock_echo_offer_invoke_returns_args() {
        let offer = EchoOffer::new();
        assert_eq!(offer.catalog_entry().id.as_str(), "test.echo");
        offer.health().await.expect("healthy");
        let resource = offer.provision(json!({})).await.expect("provision");
        assert_eq!(resource, "resource-echo-1");

        let binding = BindingId::new();
        offer.bind(binding, json!({})).await.expect("bind");

        let resp = offer
            .invoke(InvokeReq {
                binding_id: binding,
                args: json!({"n": 1}),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;

        match resp {
            InvokeResp::Ok { result, .. } => assert_eq!(result, json!({"n": 1})),
            InvokeResp::Error { code, message, .. } => {
                panic!("unexpected error {code}: {message}")
            }
        }

        offer.unbind(binding).await.expect("unbind");
    }
}
