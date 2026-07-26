//! Live `broker.health` snapshot for MCP process stores.

use std::sync::Arc;

use control::{BindingStore, CatalogRegistry, HealthSnapshot, PolicyEngine};
use serde_json::{json, Value};
use tokio::sync::Mutex;

pub(crate) struct McpHealthSnapshot {
    pub catalog: Arc<CatalogRegistry>,
    pub bindings: Arc<Mutex<BindingStore>>,
    pub policy: Arc<PolicyEngine>,
}

impl HealthSnapshot for McpHealthSnapshot {
    fn snapshot(&self) -> Value {
        let offers = self.catalog.list().len();
        let bindings = self.bindings.try_lock().map_or(0, |g| g.list().len());
        let policy = if self.policy.is_ambient() {
            "ambient"
        } else {
            "allowlist"
        };
        json!({
            "ok": true,
            "offers": offers,
            "bindings": bindings,
            "policy": policy,
        })
    }
}
