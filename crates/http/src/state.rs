//! HTTP admin shared state (`sak067-a` / `sak066-a` / sak070 Phase B / sak429-d / sak527-a).

use std::sync::{Arc, Mutex, OnceLock};

use control::{AuditLog, BindingStore, CatalogRegistry, MeterSnapshot};
use offer_compute::ComputePlane;
use rusqlite::Connection;
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
    /// Live Postgres catalog when `SAK_PERSIST_BACKEND=postgres` + URL (`sak070`).
    #[cfg(feature = "postgres")]
    pub pg_catalog: Option<Arc<PostgresCatalog>>,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: Arc::new(Mutex::new(BindingStore::new())),
            catalog: Arc::new(Mutex::new(CatalogRegistry::new())),
            invoke_count: Arc::new(Mutex::new(0)),
            compute_plane: Arc::new(OnceLock::new()),
            vault: None,
            audit: Arc::new(Mutex::new(AuditLog::new())),
            #[cfg(feature = "postgres")]
            pg_catalog: None,
        }
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
