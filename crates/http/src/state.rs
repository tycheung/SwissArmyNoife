//! HTTP admin shared state (`sak067-a` / `sak066-a` / sak070 Phase B / sak429-d / sak527-a).

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use control::{
    ApiKeyStore, AuditLog, BindRequest, BindingStore, CatalogRegistry, MeterSnapshot, Principal,
};
use offer_compute::ComputePlane;
use offer_llm::{EchoChatProvider, LlmChatOffer};
use offer_tools::ToolsLoopOffer;
use rusqlite::Connection;
use serde_json::json;
use types::OfferId;
use vault::VaultKey;

#[cfg(feature = "postgres")]
use persist_postgres::ports::PostgresCatalog;

/// Vault-backed connection store (`SQLite` + key).
pub struct VaultStore {
    pub conn: Mutex<Connection>,
    pub key: VaultKey,
}

/// Shared admin router state.
#[derive(Clone)]
pub struct AppState {
    pub bindings: Arc<Mutex<BindingStore>>,
    pub catalog: Arc<Mutex<CatalogRegistry>>,
    pub invoke_count: Arc<Mutex<u64>>,
    /// Lazy shared `SQLite` compute plane (`sak429-d`).
    pub compute_plane: Arc<OnceLock<Result<Arc<ComputePlane>, String>>>,
    /// Vault connections when `SQLite` + vault key open (`sak527-a`).
    pub vault: Option<Arc<VaultStore>>,
    /// Process-local invoke audit (`sak528-a`).
    pub audit: Arc<Mutex<AuditLog>>,
    /// Echo-backed `llm.chat` for `OpenAI` facade (`sak540-b`).
    pub llm: Arc<LlmChatOffer<EchoChatProvider>>,
    /// Default `tools.loop` for facade tool round-trips (`sak540-c`).
    pub tools_loop: Arc<ToolsLoopOffer>,
    /// Expected bearer (`MCP_HTTP_TOKEN`); `None` = no auth (`sak541-b`).
    pub http_token: Option<String>,
    /// Minted API keys accepted as bearer (`sak541-b`).
    pub api_keys: Arc<ApiKeyStore>,
    /// Live Postgres catalog when `SAK_PERSIST_BACKEND=postgres` + URL (`sak070`).
    #[cfg(feature = "postgres")]
    pub pg_catalog: Option<Arc<PostgresCatalog>>,
}

impl AppState {
    /// Build empty admin state with an echo `llm.chat` offer for the facade.
    ///
    /// # Panics
    /// If `llm.chat` / `tools.loop` catalog construction fails (fixed ids).
    #[must_use]
    pub fn new() -> Self {
        let llm =
            Arc::new(LlmChatOffer::new(EchoChatProvider, Vec::new()).expect("llm.chat catalog id"));
        let tools_loop = Arc::new(ToolsLoopOffer::with_defaults().expect("tools.loop"));
        let mut catalog = CatalogRegistry::new();
        catalog.register_offer(llm.as_ref());
        catalog.register_offer(tools_loop.as_ref());
        Self {
            bindings: Arc::new(Mutex::new(BindingStore::new())),
            catalog: Arc::new(Mutex::new(catalog)),
            invoke_count: Arc::new(Mutex::new(0)),
            compute_plane: Arc::new(OnceLock::new()),
            vault: None,
            audit: Arc::new(Mutex::new(AuditLog::new())),
            llm,
            tools_loop,
            http_token: None,
            api_keys: Arc::new(ApiKeyStore::new()),
            #[cfg(feature = "postgres")]
            pg_catalog: None,
        }
    }

    /// Require `Authorization: Bearer …` matching `token` (`sak541-b` tests).
    #[must_use]
    pub fn with_http_token(mut self, token: impl Into<String>) -> Self {
        self.http_token = Some(token.into());
        self
    }

    /// Create a short-lived `llm.chat` binding (tests / local facade demos).
    ///
    /// # Panics
    /// If the bindings mutex is poisoned.
    #[must_use]
    pub fn bind_llm_chat_for_test(&self, ttl_secs: u64) -> types::BindingId {
        self.bind_offer_for_test("llm.chat", ttl_secs, json!({}))
    }

    /// Create a short-lived `tools.loop` binding (`sak540-c`).
    ///
    /// # Panics
    /// If the bindings mutex is poisoned.
    #[must_use]
    pub fn bind_tools_loop_for_test(&self, ttl_secs: u64) -> types::BindingId {
        self.bind_offer_for_test("tools.loop", ttl_secs, json!({}))
    }

    fn bind_offer_for_test(
        &self,
        offer_id: &str,
        ttl_secs: u64,
        policy_json: serde_json::Value,
    ) -> types::BindingId {
        let mut store = self.bindings.lock().expect("bindings lock");
        store
            .bind(BindRequest {
                offer_id: OfferId::new(offer_id).expect("valid"),
                principal: Principal::local(),
                policy_json,
                ttl: Duration::from_secs(ttl_secs.max(1)),
            })
            .binding_id
    }

    /// Open (or reuse) the shared `SQLite` compute plane.
    ///
    /// # Errors
    /// DB open/migrate failures.
    pub fn compute(&self) -> Result<Arc<ComputePlane>, String> {
        match self.compute_plane.get_or_init(|| {
            ComputePlane::open_default_sqlite()
                .map(Arc::new)
                .map_err(|c| c.as_str().to_owned())
        }) {
            Ok(p) => Ok(Arc::clone(p)),
            Err(e) => Err(e.clone()),
        }
    }

    /// Build state, optionally wiring vault + Postgres catalog from env.
    #[must_use]
    pub fn from_env() -> Self {
        let mut state = Self::new();
        state.http_token = crate::auth::token_from_env();
        if state.http_token.is_some() {
            tracing::info!("http-admin bearer auth enabled (MCP_HTTP_TOKEN)");
        } else {
            tracing::warn!(
                "http-admin auth disabled — set MCP_HTTP_TOKEN (or MCP_HTTP_ALLOW_INSECURE=1)"
            );
        }
        match open_vault_store() {
            Ok(store) => {
                tracing::info!("vault connections store live");
                state.vault = Some(Arc::new(store));
            }
            Err(e) => {
                tracing::warn!(error = %e, "vault connections store unavailable");
            }
        }
        #[cfg(feature = "postgres")]
        {
            match persist_postgres::try_open_from_env() {
                Ok(Some(backend)) => {
                    tracing::info!("persist backend: postgres (catalog store live)");
                    state.pg_catalog = Some(Arc::new(backend.catalog));
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "SAK_PERSIST_BACKEND=postgres but open failed");
                }
            }
        }
        state
    }

    /// Snapshot meters for `/metrics`.
    ///
    /// # Panics
    /// If an admin mutex is poisoned.
    #[must_use]
    pub fn meter_snapshot(&self) -> MeterSnapshot {
        let bindings = self.bindings.lock().expect("bindings lock");
        let catalog = self.catalog.lock().expect("catalog lock");
        let invoke_count = *self.invoke_count.lock().expect("invoke lock");
        MeterSnapshot::new(
            invoke_count,
            bindings.list().len() as u64,
            catalog.len() as u64,
        )
    }
}

fn open_vault_store() -> Result<VaultStore, String> {
    let conn = persist_sqlite::open_default().map_err(|e| e.to_string())?;
    let key = VaultKey::bootstrap().map_err(|e| e.to_string())?;
    Ok(VaultStore {
        conn: Mutex::new(conn),
        key,
    })
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
