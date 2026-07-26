//! Concrete offer instances wired into the MCP process.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use control::{CatalogEntry, CatalogRegistry, Offer, RiskLedger};
use offer_capacity::{CapacityFitOffer, CapacityPressureOffer, CapacityProbeOffer};
use offer_compute::{ComputeNodeOffer, ComputePlane, ComputeWorkOffer};
use offer_egress::{EgressCheckOffer, EgressFetchOffer};
use offer_llm::{
    ChatProviders, EchoChatProvider, LlmChatOffer, LlmEmbedOffer, LlmOllamaManageOffer,
    LlmPreflightOffer, LlmResolveOffer, LlmTelemetryOffer,
};
use offer_memory::{
    MemoryEmbedOffer, MemoryIndexOffer, MemoryPlane, MemoryScopeOffer, MemorySearchOffer,
};
use offer_research::{ResearchBriefOffer, ResearchFetchOffer};
use offer_sandbox::{NoneBackend, SandboxExecOffer, StubBackend};
use offer_tools::ToolsRegistryOffer;
use provider_anthropic::AnthropicProvider;
use provider_core::{ChatRequest, ChatResponse, ProviderError};
use provider_ollama::OllamaProvider;
use provider_openai::OpenAiProvider;
use serde_json::Value;
use tracing::warn;
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// `LLM_BACKEND`: `ollama` (default) or `echo` (CI / no daemon).
pub const LLM_BACKEND: &str = "LLM_BACKEND";

/// `SANDBOX_BACKEND`: `none` host+jail (default) or `stub` (no spawn).
pub const SANDBOX_BACKEND: &str = "SANDBOX_BACKEND";

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

/// Host or stub sandbox offer (selected at boot).
pub enum LiveSandbox {
    Host(SandboxExecOffer<NoneBackend>),
    Stub(SandboxExecOffer<StubBackend>),
}

impl LiveSandbox {
    /// Select host or stub sandbox from `SANDBOX_BACKEND` env.
    ///
    /// # Errors
    /// Returns `SchemaInvalid` if the jail directory cannot be created or the
    /// chosen backend fails to construct.
    pub fn from_env(jail_root: &Path) -> Result<Self, ErrorCode> {
        std::fs::create_dir_all(jail_root).map_err(|e| {
            warn!(error = %e, path = %jail_root.display(), "jail mkdir failed");
            ErrorCode::SchemaInvalid
        })?;
        if std::env::var(SANDBOX_BACKEND)
            .unwrap_or_default()
            .eq_ignore_ascii_case("stub")
        {
            tracing::info!(root = %jail_root.display(), "sandbox backend=stub");
            let b = StubBackend::with_root(jail_root).map_err(|_| ErrorCode::SchemaInvalid)?;
            Ok(Self::Stub(
                SandboxExecOffer::new(b, RiskLedger::unlimited())
                    .map_err(|_| ErrorCode::SchemaInvalid)?,
            ))
        } else {
            tracing::info!(root = %jail_root.display(), "sandbox backend=none (host+jail)");
            let b = NoneBackend::with_root(jail_root).map_err(|_| ErrorCode::SchemaInvalid)?;
            Ok(Self::Host(
                SandboxExecOffer::new(b, RiskLedger::unlimited())
                    .map_err(|_| ErrorCode::SchemaInvalid)?,
            ))
        }
    }
}

impl Offer for LiveSandbox {
    fn catalog_entry(&self) -> &CatalogEntry {
        match self {
            Self::Host(o) => o.catalog_entry(),
            Self::Stub(o) => o.catalog_entry(),
        }
    }

    async fn provision(&self, params: Value) -> Result<String, ErrorCode> {
        match self {
            Self::Host(o) => o.provision(params).await,
            Self::Stub(o) => o.provision(params).await,
        }
    }

    async fn bind(&self, binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        match self {
            Self::Host(o) => o.bind(binding_id, params).await,
            Self::Stub(o) => o.bind(binding_id, params).await,
        }
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        match self {
            Self::Host(o) => o.invoke(req).await,
            Self::Stub(o) => o.invoke(req).await,
        }
    }

    async fn unbind(&self, binding_id: BindingId) -> Result<(), ErrorCode> {
        match self {
            Self::Host(o) => o.unbind(binding_id).await,
            Self::Stub(o) => o.unbind(binding_id).await,
        }
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        match self {
            Self::Host(o) => o.health().await,
            Self::Stub(o) => o.health().await,
        }
    }
}

/// Process-local runnable offers for MCP dispatch.
pub struct LiveOffers {
    pub llm: LlmChatOffer<McpLlmRouter>,
    pub llm_embed: LlmEmbedOffer<EchoChatProvider>,
    pub llm_resolve: LlmResolveOffer,
    pub llm_preflight: LlmPreflightOffer,
    pub llm_ollama_manage: LlmOllamaManageOffer,
    pub llm_telemetry: LlmTelemetryOffer,
    pub sandbox: LiveSandbox,
    pub egress: EgressCheckOffer,
    pub egress_fetch: EgressFetchOffer<offer_egress::ReqwestGet>,
    pub memory_index: MemoryIndexOffer,
    pub memory_search: MemorySearchOffer,
    pub memory_embed: MemoryEmbedOffer<EchoChatProvider>,
    pub memory_scope: MemoryScopeOffer,
    pub tools_registry: ToolsRegistryOffer,
    pub research_fetch: ResearchFetchOffer<offer_egress::ReqwestGet>,
    pub research_brief: ResearchBriefOffer,
    pub capacity_probe: CapacityProbeOffer,
    pub capacity_pressure: CapacityPressureOffer,
    pub capacity_fit: CapacityFitOffer,
    pub compute_node: ComputeNodeOffer,
    pub compute_work: ComputeWorkOffer,
}

impl LiveOffers {
    /// Build the live offer pack from process environment.
    ///
    /// # Errors
    /// Returns `SchemaInvalid` (or related codes) when sandbox, compute plane,
    /// or nested offer construction fails.
    pub fn from_env() -> Result<Self, ErrorCode> {
        let jail: PathBuf = env::config_dir().join("sandbox-jail");
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
        Ok(Self {
            llm: LlmChatOffer::new(McpLlmRouter::from_env(), vec![])?,
            // Echo embed vectors until live provider routing lands with MCP tool (sak523-b).
            llm_embed: LlmEmbedOffer::new(EchoChatProvider)?,
            llm_resolve: LlmResolveOffer::new(vec![])?,
            llm_preflight: LlmPreflightOffer::new(
                Arc::new(crate::capacity_fit::CapacityFitAdvisor::from_env()),
                reachable,
            )?,
            llm_ollama_manage: LlmOllamaManageOffer::localhost()?,
            llm_telemetry: LlmTelemetryOffer::new()?,
            sandbox: LiveSandbox::from_env(&jail)?,
            egress: EgressCheckOffer::new()?,
            egress_fetch: EgressFetchOffer::new()?,
            memory_index: MemoryIndexOffer::new(Arc::clone(&plane))?,
            memory_search: MemorySearchOffer::new(plane)?,
            memory_embed: MemoryEmbedOffer::new(EchoChatProvider)?,
            memory_scope: MemoryScopeOffer::new()?,
            tools_registry: ToolsRegistryOffer::with_defaults()?,
            research_fetch: ResearchFetchOffer::new()?,
            research_brief: ResearchBriefOffer::new()?,
            capacity_probe: CapacityProbeOffer::new(Arc::clone(&probe))?,
            capacity_pressure: CapacityPressureOffer::new(Arc::clone(&probe))?,
            capacity_fit: CapacityFitOffer::new(probe)?,
            compute_node: ComputeNodeOffer::new(Arc::clone(&compute))?,
            compute_work: ComputeWorkOffer::new(compute)?,
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
        catalog.register_offer(&self.egress);
        catalog.register_offer(&self.egress_fetch);
        catalog.register_offer(&self.memory_index);
        catalog.register_offer(&self.memory_search);
        catalog.register_offer(&self.memory_embed);
        catalog.register_offer(&self.memory_scope);
        catalog.register_offer(&self.tools_registry);
        catalog.register_offer(&self.research_fetch);
        catalog.register_offer(&self.research_brief);
        catalog.register_offer(&self.capacity_probe);
        catalog.register_offer(&self.capacity_pressure);
        catalog.register_offer(&self.capacity_fit);
        catalog.register_offer(&self.compute_node);
        catalog.register_offer(&self.compute_work);
        catalog
    }
}
