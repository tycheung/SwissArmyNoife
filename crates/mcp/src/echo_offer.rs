//! Fallback offer for unknown / expired binding dispatch paths only.

use control::{CatalogEntry, Offer};
use serde_json::Value;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

#[derive(Clone, Debug)]
pub struct EchoOffer {
    entry: CatalogEntry,
}

impl EchoOffer {
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when `id` is empty.
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new(id, version)?,
        })
    }
}

impl Offer for EchoOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok(format!("res-{}", self.entry.id.as_str()))
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
