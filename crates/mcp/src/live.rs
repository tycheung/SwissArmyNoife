//! Concrete offer instances wired into the MCP process.

use std::path::PathBuf;
use std::sync::Arc;

use control::CatalogRegistry;
use offer_browser::BrowserSessionOffer;
use offer_capacity::{CapacityFitOffer, CapacityPressureOffer, CapacityProbeOffer};
use offer_compute::{ComputeNodeOffer, ComputePlane, ComputeWorkOffer};
use offer_egress::{EgressCheckOffer, EgressFetchOffer};
use offer_eval::EvalRunOffer;
use offer_llm::{
    ConnectionRef, LlmChatOffer, LlmEmbedOffer, LlmOllamaManageOffer, LlmPreflightOffer,
    LlmResolveOffer, LlmTelemetryOffer,
};
use offer_memory::{
    MemoryEmbedOffer, MemoryIndexOffer, MemoryPlane, MemoryScopeOffer, MemorySearchOffer,
};
use offer_research::{ResearchBriefOffer, ResearchFetchOffer};
use offer_sandbox::{FilesystemJail, SandboxJailOffer};
use offer_tools::{ToolsLoopOffer, ToolsRegistryOffer};
use types::ErrorCode;

pub use crate::live_llm::{McpLlmRouter, RouterAsLlm, LLM_BACKEND};
pub use crate::live_sandbox::{LiveSandbox, SANDBOX_BACKEND};

/// Process-local runnable offers for MCP dispatch.
pub struct LiveOffers {
    pub llm: LlmChatOffer<McpLlmRouter>,
    pub llm_embed: LlmEmbedOffer<McpLlmRouter>,
    pub llm_resolve: LlmResolveOffer,
    pub llm_preflight: LlmPreflightOffer,
    pub llm_ollama_manage: LlmOllamaManageOffer,
    pub llm_telemetry: LlmTelemetryOffer,
    pub sandbox: LiveSandbox,
    pub sandbox_jail: SandboxJailOffer,
    pub egress: EgressCheckOffer,
    pub egress_fetch: EgressFetchOffer<offer_egress::ReqwestGet>,
    pub memory_index: MemoryIndexOffer,
    pub memory_search: MemorySearchOffer,
    pub memory_embed: MemoryEmbedOffer<RouterAsLlm>,
    pub memory_scope: MemoryScopeOffer,
    pub tools_registry: ToolsRegistryOffer,
    pub tools_loop: ToolsLoopOffer,
    pub research_fetch: ResearchFetchOffer<offer_egress::ReqwestGet>,
    pub research_brief: ResearchBriefOffer,
    pub browser: BrowserSessionOffer,
    pub capacity_probe: CapacityProbeOffer,
    pub capacity_pressure: CapacityPressureOffer,
    pub capacity_fit: CapacityFitOffer,
    pub compute_node: ComputeNodeOffer,
    pub compute_work: ComputeWorkOffer,
    pub eval_run: EvalRunOffer,
}

impl LiveOffers {
    /// Build the live offer pack from process environment.
    ///
    /// # Errors
    /// Returns `SchemaInvalid` (or related codes) when sandbox, compute plane,
    /// or nested offer construction fails.
    pub fn from_env() -> Result<Self, ErrorCode> {
        let jail: PathBuf = env::config_dir().join("sandbox-jail");
        std::fs::create_dir_all(&jail).map_err(|_| ErrorCode::SchemaInvalid)?;
        let jail_fs = FilesystemJail::new(&jail).map_err(|_| ErrorCode::SchemaInvalid)?;
        let plane = Arc::new(MemoryPlane::new());
        let compute = Arc::new(ComputePlane::from_env()?);
        let probe: Arc<dyn offer_capacity::HardwareProbe> =
            Arc::from(offer_capacity::probe_from_env());
        let reachable = if std::env::var(LLM_BACKEND)
            .unwrap_or_default()
            .eq_ignore_ascii_case("echo")
        {
            vec!["echo".into()]
        } else {
            vec![
                "ollama".into(),
                "openai".into(),
                "anthropic".into(),
                "echo".into(),
            ]
        };
        let connections = vault_connection_refs().unwrap_or_else(|code| {
            tracing::warn!(code = code.as_str(), "vault connection catalog unavailable");
            Vec::new()
        });
        let sandbox = LiveSandbox::from_env(&jail)?;
        let sandbox_jail = SandboxJailOffer::new(jail_fs)?.with_backend(sandbox.backend_label());
        Ok(Self {
            llm: LlmChatOffer::new(McpLlmRouter::from_env(), connections.clone())?,
            llm_embed: LlmEmbedOffer::new(McpLlmRouter::from_env())?,
            llm_resolve: LlmResolveOffer::new(connections)?,
            llm_preflight: LlmPreflightOffer::new(
                Arc::new(crate::capacity_fit::CapacityFitAdvisor::from_env()),
                reachable,
            )?,
            llm_ollama_manage: LlmOllamaManageOffer::localhost()?,
            llm_telemetry: LlmTelemetryOffer::new()?,
            sandbox,
            sandbox_jail,
            egress: EgressCheckOffer::new()?,
            egress_fetch: EgressFetchOffer::new()?,
            memory_index: MemoryIndexOffer::new(Arc::clone(&plane))?,
            memory_search: MemorySearchOffer::new(plane)?,
            memory_embed: MemoryEmbedOffer::new(RouterAsLlm(McpLlmRouter::from_env()))?,
            memory_scope: MemoryScopeOffer::new()?,
            tools_registry: ToolsRegistryOffer::with_defaults()?,
            tools_loop: ToolsLoopOffer::with_defaults()?,
            research_fetch: ResearchFetchOffer::new()?,
            research_brief: ResearchBriefOffer::new()?,
            browser: BrowserSessionOffer::from_env(env::config_dir().join("browser"))?,
            capacity_probe: CapacityProbeOffer::new(Arc::clone(&probe))?,
            capacity_pressure: CapacityPressureOffer::new(Arc::clone(&probe))?,
            capacity_fit: CapacityFitOffer::new(probe)?,
            compute_node: ComputeNodeOffer::new(Arc::clone(&compute))?,
            compute_work: ComputeWorkOffer::new(compute)?,
            eval_run: EvalRunOffer::new()?,
        })
    }

    pub fn seed_catalog(&self) -> CatalogRegistry {
        let mut catalog = CatalogRegistry::new();
        catalog.register_offer(&self.llm);
        catalog.register_offer(&self.llm_embed);
        catalog.register_offer(&self.llm_resolve);
        catalog.register_offer(&self.llm_preflight);
        catalog.register_offer(&self.llm_ollama_manage);
        catalog.register_offer(&self.llm_telemetry);
        catalog.register_offer(&self.sandbox);
        catalog.register_offer(&self.sandbox_jail);
        catalog.register_offer(&self.egress);
        catalog.register_offer(&self.egress_fetch);
        catalog.register_offer(&self.memory_index);
        catalog.register_offer(&self.memory_search);
        catalog.register_offer(&self.memory_embed);
        catalog.register_offer(&self.memory_scope);
        catalog.register_offer(&self.tools_registry);
        catalog.register_offer(&self.tools_loop);
        catalog.register_offer(&self.research_fetch);
        catalog.register_offer(&self.research_brief);
        catalog.register_offer(&self.browser);
        catalog.register_offer(&self.capacity_probe);
        catalog.register_offer(&self.capacity_pressure);
        catalog.register_offer(&self.capacity_fit);
        catalog.register_offer(&self.compute_node);
        catalog.register_offer(&self.compute_work);
        catalog.register_offer(&self.eval_run);
        catalog
    }
}

/// Load vault connection metadata for LLM resolve (no secrets).
///
/// # Errors
/// [`ErrorCode::VaultMissing`] when the `SQLite` vault cannot be opened or listed.
pub(crate) fn vault_connection_refs() -> Result<Vec<ConnectionRef>, ErrorCode> {
    #[cfg(test)]
    if FORCE_VAULT_MISS.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(ErrorCode::VaultMissing);
    }
    let conn = persist_sqlite::open_default().map_err(|_| ErrorCode::VaultMissing)?;
    persist_sqlite::list_connections(&conn)
        .map_err(|_| ErrorCode::VaultMissing)
        .map(|rows| {
            rows.into_iter()
                .map(|m| ConnectionRef {
                    connection_id: m.connection_id,
                    provider: m.provider,
                    label: m.label,
                })
                .collect()
        })
}

#[cfg(test)]
static FORCE_VAULT_MISS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn force_vault_miss(on: bool) {
    FORCE_VAULT_MISS.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use vault::SecretString;

    #[test]
    fn vault_refs_load_when_sqlite_has_rows() {
        let _g = crate::MCP_TEST_ENV_LOCK.lock().expect("lock");
        let tmp = tempfile::tempdir().expect("tmp");
        std::env::set_var(persist_sqlite::CONFIG_DIR, tmp.path());
        std::env::set_var(
            vault::VAULT_KEY,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        let conn = persist_sqlite::open_default().expect("db");
        let key = vault::VaultKey::bootstrap().expect("key");
        persist_sqlite::put_connection(
            &conn,
            &key,
            "conn-live",
            "openai",
            "prod",
            &SecretString::new("sk-test"),
        )
        .expect("put");
        let refs = vault_connection_refs().expect("refs");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].connection_id, "conn-live");
        assert_eq!(refs[0].provider, "openai");
        let dbg = format!("{refs:?}");
        assert!(!dbg.contains("sk-test"));
        std::env::remove_var(persist_sqlite::CONFIG_DIR);
        std::env::remove_var(vault::VAULT_KEY);
    }

    #[test]
    fn vault_refs_db_miss_is_vault_missing() {
        let _g = crate::MCP_TEST_ENV_LOCK.lock().expect("lock");
        force_vault_miss(true);
        let err = vault_connection_refs().expect_err("db miss");
        force_vault_miss(false);
        assert_eq!(err, ErrorCode::VaultMissing);
    }
}
