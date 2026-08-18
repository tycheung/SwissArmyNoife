//! Host / stub / docker sandbox offer selected at MCP boot.

use std::path::Path;

use control::{CatalogEntry, Offer, RiskLedger};
use offer_sandbox::{DockerBackend, NoneBackend, SandboxExecOffer, StubBackend};
use serde_json::Value;
use tracing::warn;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// `SANDBOX_BACKEND`: `none` host+jail (default), `stub` (no spawn), or `docker`.
pub const SANDBOX_BACKEND: &str = "SANDBOX_BACKEND";

/// Host, stub, or docker sandbox offer (selected at boot).
pub enum LiveSandbox {
    Host(SandboxExecOffer<NoneBackend>),
    Stub(SandboxExecOffer<StubBackend>),
    Docker(SandboxExecOffer<DockerBackend>),
}

impl LiveSandbox {
    /// Select host, stub, or docker sandbox from `SANDBOX_BACKEND` env.
    ///
    /// # Errors
    /// Returns `SchemaInvalid` if the jail directory cannot be created or the
    /// chosen backend fails to construct.
    pub fn from_env(jail_root: &Path) -> Result<Self, ErrorCode> {
        std::fs::create_dir_all(jail_root).map_err(|e| {
            warn!(error = %e, path = %jail_root.display(), "jail mkdir failed");
            ErrorCode::SchemaInvalid
        })?;
        let raw = std::env::var(SANDBOX_BACKEND).unwrap_or_default();
        match raw.to_ascii_lowercase().as_str() {
            "stub" => {
                tracing::info!(root = %jail_root.display(), "sandbox backend=stub");
                let b = StubBackend::with_root(jail_root).map_err(|_| ErrorCode::SchemaInvalid)?;
                Ok(Self::Stub(
                    SandboxExecOffer::new(b, RiskLedger::unlimited())
                        .map_err(|_| ErrorCode::SchemaInvalid)?,
                ))
            }
            "docker" => {
                tracing::info!(root = %jail_root.display(), "sandbox backend=docker");
                let b =
                    DockerBackend::with_root(jail_root).map_err(|_| ErrorCode::SchemaInvalid)?;
                Ok(Self::Docker(
                    SandboxExecOffer::new(b, RiskLedger::unlimited())
                        .map_err(|_| ErrorCode::SchemaInvalid)?,
                ))
            }
            _ => {
                tracing::info!(root = %jail_root.display(), "sandbox backend=none (host+jail)");
                let b = NoneBackend::with_root(jail_root).map_err(|_| ErrorCode::SchemaInvalid)?;
                Ok(Self::Host(
                    SandboxExecOffer::new(b, RiskLedger::unlimited())
                        .map_err(|_| ErrorCode::SchemaInvalid)?,
                ))
            }
        }
    }

    /// Env label for the selected backend (`none` / `stub` / `docker`).
    #[must_use]
    pub const fn backend_label(&self) -> &'static str {
        match self {
            Self::Host(_) => "none",
            Self::Stub(_) => "stub",
            Self::Docker(_) => "docker",
        }
    }
}

impl Offer for LiveSandbox {
    fn catalog_entry(&self) -> &CatalogEntry {
        match self {
            Self::Host(o) => o.catalog_entry(),
            Self::Stub(o) => o.catalog_entry(),
            Self::Docker(o) => o.catalog_entry(),
        }
    }

    async fn provision(&self, params: Value) -> Result<String, ErrorCode> {
        match self {
            Self::Host(o) => o.provision(params).await,
            Self::Stub(o) => o.provision(params).await,
            Self::Docker(o) => o.provision(params).await,
        }
    }

    async fn bind(&self, binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        match self {
            Self::Host(o) => o.bind(binding_id, params).await,
            Self::Stub(o) => o.bind(binding_id, params).await,
            Self::Docker(o) => o.bind(binding_id, params).await,
        }
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        match self {
            Self::Host(o) => o.invoke(req).await,
            Self::Stub(o) => o.invoke(req).await,
            Self::Docker(o) => o.invoke(req).await,
        }
    }

    async fn unbind(&self, binding_id: BindingId) -> Result<(), ErrorCode> {
        match self {
            Self::Host(o) => o.unbind(binding_id).await,
            Self::Stub(o) => o.unbind(binding_id).await,
            Self::Docker(o) => o.unbind(binding_id).await,
        }
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        match self {
            Self::Host(o) => o.health().await,
            Self::Stub(o) => o.health().await,
            Self::Docker(o) => o.health().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_env_selects(label: &str) -> LiveSandbox {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().expect("env lock");
        let tmp = tempfile::tempdir().expect("tmp");
        std::env::set_var(SANDBOX_BACKEND, label);
        let live = LiveSandbox::from_env(tmp.path()).expect("sandbox");
        std::env::remove_var(SANDBOX_BACKEND);
        live
    }

    #[test]
    fn from_env_docker_selects_docker_backend() {
        let live = from_env_selects("docker");
        assert_eq!(live.backend_label(), "docker");
        assert!(matches!(live, LiveSandbox::Docker(_)));
    }

    #[test]
    fn from_env_stub_selects_stub_backend() {
        let live = from_env_selects("stub");
        assert_eq!(live.backend_label(), "stub");
        assert!(matches!(live, LiveSandbox::Stub(_)));
    }

    #[test]
    fn from_env_none_selects_host_backend() {
        let live = from_env_selects("none");
        assert_eq!(live.backend_label(), "none");
        assert!(matches!(live, LiveSandbox::Host(_)));
    }
}
