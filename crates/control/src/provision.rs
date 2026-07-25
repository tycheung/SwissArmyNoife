//! Provision verb: allocate offer resources with a small state machine.

use std::collections::HashMap;

use types::{BindingId, ErrorCode, OfferId};

/// Lifecycle state for a provisioned resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceState {
    /// Allocation in progress (reserved for async providers).
    Provisioning,
    /// Ready to bind / invoke.
    Ready,
    /// Provider failed; not usable.
    Failed,
    /// Released after unbind / teardown.
    Released,
}

impl ResourceState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Released => "released",
        }
    }
}

/// One provisioned resource handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRecord {
    pub resource_id: String,
    pub offer_id: OfferId,
    pub state: ResourceState,
    pub detail: Option<String>,
}

/// Process-local provision table.
#[derive(Debug, Default)]
pub struct ProvisionStore {
    resources: HashMap<String, ResourceRecord>,
}

impl ProvisionStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a resource for `offer_id` and mark it [`ResourceState::Ready`] (sync v0).
    pub fn provision(&mut self, offer_id: OfferId) -> ResourceRecord {
        let resource_id = format!("res-{}-{}", offer_id.as_str(), BindingId::new());
        let record = ResourceRecord {
            resource_id: resource_id.clone(),
            offer_id,
            state: ResourceState::Ready,
            detail: None,
        };
        self.resources.insert(resource_id, record.clone());
        record
    }

    /// Lookup by resource id.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when the id is unknown.
    pub fn get(&self, resource_id: &str) -> Result<&ResourceRecord, ErrorCode> {
        self.resources
            .get(resource_id)
            .ok_or(ErrorCode::SchemaInvalid)
    }

    /// Transition Ready → Failed (provider error path).
    ///
    /// # Errors
    /// Unknown id → [`ErrorCode::SchemaInvalid`]; wrong state → [`ErrorCode::SchemaInvalid`].
    pub fn mark_failed(
        &mut self,
        resource_id: &str,
        detail: impl Into<String>,
    ) -> Result<&ResourceRecord, ErrorCode> {
        let record = self
            .resources
            .get_mut(resource_id)
            .ok_or(ErrorCode::SchemaInvalid)?;
        if record.state != ResourceState::Ready && record.state != ResourceState::Provisioning {
            return Err(ErrorCode::SchemaInvalid);
        }
        record.state = ResourceState::Failed;
        record.detail = Some(detail.into());
        Ok(record)
    }

    /// Transition Ready/Failed → Released.
    ///
    /// # Errors
    /// Unknown id → [`ErrorCode::SchemaInvalid`]; already released → [`ErrorCode::SchemaInvalid`].
    pub fn release(&mut self, resource_id: &str) -> Result<&ResourceRecord, ErrorCode> {
        let record = self
            .resources
            .get_mut(resource_id)
            .ok_or(ErrorCode::SchemaInvalid)?;
        if record.state == ResourceState::Released {
            return Err(ErrorCode::SchemaInvalid);
        }
        record.state = ResourceState::Released;
        Ok(record)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provision_ready_then_release() {
        let mut store = ProvisionStore::new();
        let record = store.provision(OfferId::new("llm.chat").expect("valid"));
        assert_eq!(record.state, ResourceState::Ready);
        assert_eq!(record.state.as_str(), "ready");

        let live = store.get(&record.resource_id).expect("get");
        assert_eq!(live.offer_id.as_str(), "llm.chat");

        let released = store.release(&record.resource_id).expect("release");
        assert_eq!(released.state, ResourceState::Released);
        assert_eq!(
            store.release(&record.resource_id),
            Err(ErrorCode::SchemaInvalid)
        );
    }

    #[test]
    fn mark_failed_from_ready() {
        let mut store = ProvisionStore::new();
        let record = store.provision(OfferId::new("sandbox.exec").expect("valid"));
        let failed = store
            .mark_failed(&record.resource_id, "provider down")
            .expect("fail");
        assert_eq!(failed.state, ResourceState::Failed);
        assert_eq!(failed.detail.as_deref(), Some("provider down"));
    }

    #[test]
    fn get_missing_is_schema_invalid() {
        let store = ProvisionStore::new();
        assert_eq!(store.get("missing"), Err(ErrorCode::SchemaInvalid));
    }
}
