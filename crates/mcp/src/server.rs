//! MCP server state, tools, and handlers.

use std::sync::Arc;
use std::time::Duration;

use crate::live::LiveOffers;
use crate::resources::{list_resources, read_resource};
use crate::session::bind_pack;
use crate::tool_args::{
    AuditQueryArgs, BindArgs, CapacityFitArgs, CapacityPressureArgs, CapacityProbeArgs,
    CatalogGetArgs, ComputeNodeArgs, ComputeWorkArgs, EgressCheckArgs, EgressFetchArgs, FsEditArgs,
    FsGrepArgs, FsReadArgs, FsWriteArgs, InvokeArgs, LlmChatToolArgs, LlmEmbedArgs,
    LlmPreflightArgs, MemoryEmbedArgs, MemoryIndexArgs, MemoryScopeArgs, MemorySearchArgs,
    ModuleInvokeArgs, OllamaManageArgs, ProvisionArgs, ResearchBriefArgs, ResearchFetchArgs,
    SandboxExecToolArgs, SandboxJailArgs, SessionBindArgs, ShellExecArgs, TelemetryArgs,
    ToolsLoopArgs, ToolsRegistryArgs, UnbindArgs,
};
use crate::util::{expires_unix, parse_binding_id, serialize_resp};
use crate::workspace_tools::boot_fs_shell;
use control::{
    resolve_policy, ApiKeyStore, AuditLog, BindRequest, BindingStore, BrokerHealthOffer,
    IdempotencyStore, PolicyEngine, ProvisionStore, RateLimiter,
};
use module_registry::ModuleRuntime;
use offer_tools::{FsTools, HostShellRunner, ShellTools};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{PaginatedRequestParam, ReadResourceRequestParam, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use serde_json::json;
use tokio::sync::Mutex;
use types::OfferId;

use crate::health_snap::McpHealthSnapshot;
use crate::progress::notify_progress;

#[derive(Clone)]
pub struct McpServer {
    pub(crate) tool_router: ToolRouter<Self>,
    pub(crate) catalog: Arc<control::CatalogRegistry>,
    pub(crate) bindings: Arc<Mutex<BindingStore>>,
    pub(crate) provisions: Arc<Mutex<ProvisionStore>>,
    pub(crate) policy: Arc<PolicyEngine>,
    pub(crate) audit: Arc<Mutex<AuditLog>>,
    pub(crate) offers: Arc<LiveOffers>,
    pub(crate) broker_health: Arc<BrokerHealthOffer>,
    /// Shared with HTTP auth middleware (`sak059-c`); populated on Streamable HTTP boot.
    #[allow(dead_code)]
    pub(crate) api_keys: Arc<ApiKeyStore>,
    pub(crate) fs: Arc<FsTools>,
    pub(crate) shell: Arc<ShellTools<HostShellRunner>>,
    pub(crate) modules: Arc<ModuleRuntime>,
    pub(crate) idempotency: Arc<std::sync::Mutex<IdempotencyStore>>,
    pub(crate) rate_limiter: Arc<std::sync::Mutex<RateLimiter>>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl McpServer {
    /// Boot catalog, live offers, and workspace tools from process env.
    #[must_use]
    pub fn new() -> Self {
        Self::with_api_keys(Arc::new(ApiKeyStore::new()))
    }

    /// Same as [`Self::new`] but shares the given API key store (HTTP auth).
    ///
    /// # Panics
    /// Panics if live offers or catalog seed fail at boot (`expect` on env setup).
    #[must_use]
    pub fn with_api_keys(api_keys: Arc<ApiKeyStore>) -> Self {
        let offers = Arc::new(LiveOffers::from_env().expect("live offers boot"));
        let mut catalog = offers.seed_catalog();
        catalog.register(
            control::CatalogEntry::new("broker.health", "0.1.0").expect("broker.health id"),
        );
        let catalog = Arc::new(catalog);
        let bindings = Arc::new(Mutex::new(BindingStore::new()));
        let policy = Arc::new(PolicyEngine::ambient());
        let broker_health = Arc::new(
            BrokerHealthOffer::new(Arc::new(McpHealthSnapshot {
                catalog: Arc::clone(&catalog),
                bindings: Arc::clone(&bindings),
                policy: Arc::clone(&policy),
            }))
            .expect("broker.health offer"),
        );
        let (fs, shell) = boot_fs_shell().expect("workspace tools boot");
        Self {
            tool_router: Self::tool_router(),
            catalog,
            bindings,
            provisions: Arc::new(Mutex::new(ProvisionStore::new())),
            policy,
            audit: Arc::new(Mutex::new(AuditLog::new())),
            offers,
            broker_health,
            api_keys,
            fs: Arc::new(fs),
            shell: Arc::new(shell),
            modules: Arc::new(ModuleRuntime::new()),
            idempotency: Arc::new(std::sync::Mutex::new(IdempotencyStore::default_bind())),
            rate_limiter: Arc::new(std::sync::Mutex::new(RateLimiter::from_env())),
        }
    }

    /// Liveness probe for MCP clients.
    #[tool(description = "SwissArmyNoife liveness probe — returns ok")]
    async fn ping(&self) -> Result<String, McpError> {
        Ok("ok".into())
    }

    /// Control-plane health snapshot (`broker.health`).
    #[tool(description = "Broker control-plane health (offers/bindings/policy)")]
    async fn broker_health(&self) -> Result<String, McpError> {
        Ok(self.broker_health.snapshot_json().to_string())
    }

    /// List registered capability offers (`catalog.list`).
    #[tool(description = "List catalogued SwissArmyNoife offers (id + version)")]
    async fn catalog_list(&self) -> Result<String, McpError> {
        let offers: Vec<_> = self
            .catalog
            .list()
            .into_iter()
            .map(|e| {
                json!({
                    "id": e.id.as_str(),
                    "version": e.version,
                })
            })
            .collect();
        Ok(json!({ "offers": offers }).to_string())
    }

    /// List vault connection metadata only (no secrets; ambient-safe).
    #[tool(description = "List vault connection metadata (id/provider/label; no secrets)")]
    async fn connections_list(&self) -> Result<String, McpError> {
        let connections: Vec<_> = crate::live::vault_connection_refs()
            .into_iter()
            .map(|c| {
                json!({
                    "connection_id": c.connection_id,
                    "provider": c.provider,
                    "label": c.label,
                })
            })
            .collect();
        Ok(json!({ "connections": connections }).to_string())
    }

    /// Query redacted invoke audit events (`sak528-b`).
    #[tool(description = "Query redacted invoke audit events (optional offer_id/since)")]
    async fn audit_query(
        &self,
        Parameters(args): Parameters<AuditQueryArgs>,
    ) -> Result<String, McpError> {
        use std::time::{Duration, UNIX_EPOCH};
        let since = args.since.map(|s| UNIX_EPOCH + Duration::from_secs(s));
        let audit = self.audit.lock().await;
        let events: Vec<_> = audit
            .query(args.offer_id.as_deref(), since)
            .into_iter()
            .map(|ev| {
                let created_at = ev
                    .created_at
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                json!({
                    "invoke_id": ev.invoke_id.to_string(),
                    "binding_id": ev.binding_id.to_string(),
                    "offer_id": ev.offer_id.as_str(),
                    "status": ev.status.as_str(),
                    "code": ev.code.map(|c| c.as_str().to_owned()),
                    "detail": ev.detail,
                    "created_at": created_at,
                })
            })
            .collect();
        Ok(json!({ "events": events }).to_string())
    }

    /// Fetch one offer by id (`catalog.get`).
    #[tool(description = "Get a single catalogued offer by id")]
    async fn catalog_get(
        &self,
        Parameters(CatalogGetArgs { offer_id }): Parameters<CatalogGetArgs>,
    ) -> Result<String, McpError> {
        let id = OfferId::new(offer_id)
            .map_err(|code| McpError::invalid_params(format!("{code}: invalid offer_id"), None))?;
        let entry = self.catalog.get(&id).map_err(|code| {
            McpError::invalid_params(format!("{code}: offer not in catalog"), None)
        })?;
        Ok(json!({
            "id": entry.id.as_str(),
            "version": entry.version,
        })
        .to_string())
    }

    /// Allocate a provider resource for an offer (`provision`).
    #[tool(description = "Provision resources for a catalogued offer")]
    async fn provision(
        &self,
        Parameters(ProvisionArgs {
            offer_id,
            idempotency_key,
        }): Parameters<ProvisionArgs>,
    ) -> Result<String, McpError> {
        let id = OfferId::new(offer_id)
            .map_err(|code| McpError::invalid_params(format!("{code}: invalid offer_id"), None))?;
        self.catalog.get(&id).map_err(|code| {
            McpError::invalid_params(format!("{code}: offer not in catalog"), None)
        })?;

        let fingerprint = IdempotencyStore::provision_fingerprint(&id);
        let now = std::time::SystemTime::now();

        if let Some(ref key) = idempotency_key {
            let replay = self
                .idempotency
                .lock()
                .expect("idempotency lock")
                .lookup_provision(key, &fingerprint, now)
                .map_err(|code| {
                    McpError::invalid_params(format!("{code}: idempotency conflict"), None)
                })?;
            if let Some(resource_id) = replay {
                let store = self.provisions.lock().await;
                if let Ok(record) = store.get(&resource_id) {
                    return Ok(json!({
                        "offer_id": record.offer_id.as_str(),
                        "resource_id": record.resource_id,
                        "state": record.state.as_str(),
                        "idempotent_replay": true,
                    })
                    .to_string());
                }
            }
        }

        let mut store = self.provisions.lock().await;
        let record = store.provision(id);
        if let Some(ref key) = idempotency_key {
            self.idempotency
                .lock()
                .expect("idempotency lock")
                .record_provision(key, &fingerprint, &record.resource_id, now);
        }
        Ok(json!({
            "offer_id": record.offer_id.as_str(),
            "resource_id": record.resource_id,
            "state": record.state.as_str(),
        })
        .to_string())
    }

    /// Bind principal + policy to an offer (`bind`).
    #[tool(description = "Create a TTL-scoped binding for an offer")]
    async fn bind(
        &self,
        Parameters(BindArgs {
            offer_id,
            principal,
            policy,
            policy_template,
            idempotency_key,
            ttl_secs,
        }): Parameters<BindArgs>,
    ) -> Result<String, McpError> {
        let id = OfferId::new(offer_id)
            .map_err(|code| McpError::invalid_params(format!("{code}: invalid offer_id"), None))?;
        self.catalog.get(&id).map_err(|code| {
            McpError::invalid_params(format!("{code}: offer not in catalog"), None)
        })?;
        let principal = control::Principal::from_bind_arg(&principal);

        self.rate_limiter
            .lock()
            .expect("rate limiter lock")
            .check(principal.as_str())
            .map_err(|_| McpError::invalid_params(RateLimiter::deny_message(), None))?;

        self.policy
            .check(principal.as_str(), &id)
            .map_err(|code| McpError::invalid_params(format!("{code}: principal denied"), None))?;

        let has_template = policy_template.is_some();
        let has_inline = !policy.is_null() && policy != json!({});
        if has_template && has_inline {
            return Err(McpError::invalid_params(
                format!(
                    "{}: policy_template and policy are mutually exclusive",
                    types::ErrorCode::SchemaInvalid.as_str()
                ),
                None,
            ));
        }
        let policy_json = resolve_policy(
            policy_template.as_deref(),
            if has_inline {
                Some(policy.clone())
            } else {
                None
            },
        )
        .map_err(|code| McpError::invalid_params(format!("{code}: policy resolve failed"), None))?;

        let fingerprint = IdempotencyStore::bind_fingerprint(&id, &principal, &policy_json);
        let now = std::time::SystemTime::now();

        if let Some(ref key) = idempotency_key {
            let replay = self
                .idempotency
                .lock()
                .expect("idempotency lock")
                .lookup(key, &fingerprint, now)
                .map_err(|code| {
                    McpError::invalid_params(format!("{code}: idempotency conflict"), None)
                })?;
            if let Some(existing_id) = replay {
                let store = self.bindings.lock().await;
                if let Ok(record) = store.get(existing_id) {
                    return Ok(json!({
                        "binding_id": record.binding_id.to_string(),
                        "offer_id": record.offer_id.as_str(),
                        "principal": record.principal.as_str(),
                        "principal_kind": record.principal.kind.as_str(),
                        "expires_at": expires_unix(record.expires_at),
                        "idempotent_replay": true,
                    })
                    .to_string());
                }
            }
        }

        let mut store = self.bindings.lock().await;
        let record = store.bind(BindRequest {
            offer_id: id,
            principal: principal.clone(),
            policy_json: policy_json.clone(),
            ttl: Duration::from_secs(ttl_secs),
        });
        if let Some(ref key) = idempotency_key {
            self.idempotency.lock().expect("idempotency lock").record(
                key,
                &fingerprint,
                record.binding_id,
                now,
            );
        }
        drop(store);
        self.apply_offer_bind(
            record.offer_id.as_str(),
            record.binding_id,
            policy_json,
            record.principal.as_str(),
        )
        .await?;
        Ok(json!({
            "binding_id": record.binding_id.to_string(),
            "offer_id": record.offer_id.as_str(),
            "principal": record.principal.as_str(),
            "principal_kind": record.principal.kind.as_str(),
            "expires_at": expires_unix(record.expires_at),
        })
        .to_string())
    }

    /// Release a binding (`unbind`).
    #[tool(description = "Unbind and release a binding id")]
    async fn unbind(
        &self,
        Parameters(UnbindArgs { binding_id }): Parameters<UnbindArgs>,
    ) -> Result<String, McpError> {
        let id = parse_binding_id(&binding_id)?;
        let mut store = self.bindings.lock().await;
        let removed = store.unbind(id).map_err(|code| {
            McpError::invalid_params(format!("{code}: binding missing or expired"), None)
        })?;
        let _ = self
            .apply_offer_unbind(removed.offer_id.as_str(), removed.binding_id)
            .await;
        Ok(json!({
            "binding_id": removed.binding_id.to_string(),
            "offer_id": removed.offer_id.as_str(),
            "unbound": true,
        })
        .to_string())
    }

    /// Bind several offers at once (`sak114` session pack).
    #[tool(description = "Bind a pack of offers with shared principal/TTL/policy")]
    async fn session_bind(
        &self,
        Parameters(SessionBindArgs {
            offer_ids,
            principal,
            policy,
            ttl_secs,
        }): Parameters<SessionBindArgs>,
    ) -> Result<String, McpError> {
        if offer_ids.is_empty() {
            return Err(McpError::invalid_params("offer_ids empty", None));
        }
        let principal = control::Principal::from_bind_arg(principal.as_deref().unwrap_or("local"));
        let policy = policy.unwrap_or_else(|| json!({}));
        let ttl_secs = ttl_secs.unwrap_or(300);
        for raw in &offer_ids {
            let id = OfferId::new(raw.clone()).map_err(|code| {
                McpError::invalid_params(format!("{code}: invalid offer_id"), None)
            })?;
            self.catalog.get(&id).map_err(|code| {
                McpError::invalid_params(format!("{code}: offer not in catalog"), None)
            })?;
            self.policy.check(principal.as_str(), &id).map_err(|code| {
                McpError::invalid_params(format!("{code}: principal denied"), None)
            })?;
        }
        let mut store = self.bindings.lock().await;
        let pack = bind_pack(
            &mut store,
            &self.policy,
            &offer_ids,
            &principal,
            ttl_secs,
            &policy,
        )
        .map_err(|code| McpError::invalid_params(format!("{code}: bind_pack failed"), None))?;
        drop(store);
        let mut bindings = Vec::with_capacity(pack.len());
        for (offer_id, binding_id) in &pack {
            self.apply_offer_bind(offer_id, *binding_id, policy.clone(), principal.as_str())
                .await?;
            bindings.push(json!({
                "offer_id": offer_id,
                "binding_id": binding_id.to_string(),
            }));
        }
        Ok(json!({ "bindings": bindings }).to_string())
    }

    /// Invoke against a live binding (`invoke`) via the control-plane dispatcher.
    #[tool(description = "Invoke a bound offer with JSON args (returns InvokeResp)")]
    async fn invoke(
        &self,
        Parameters(InvokeArgs {
            binding_id,
            args,
            offer,
        }): Parameters<InvokeArgs>,
    ) -> Result<String, McpError> {
        let binding_id = parse_binding_id(&binding_id)?;
        let offer_claim = match offer {
            Some(raw) => Some(
                OfferId::new(raw)
                    .map_err(|code| McpError::invalid_params(format!("{code}: bad offer"), None))?,
            ),
            None => None,
        };
        let resp = self.dispatch_invoke(binding_id, args, offer_claim).await?;
        serialize_resp(&resp)
    }

    /// Typed invoke for `llm.chat` (returns `InvokeResp` JSON).
    #[tool(description = "Chat via bound llm.chat offer (returns InvokeResp)")]
    async fn llm_chat(
        &self,
        Parameters(args): Parameters<LlmChatToolArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<String, McpError> {
        notify_progress(&context, 0.0, Some(1.0), "llm_chat start").await;
        let out = self.llm_chat_inner(args).await?;
        notify_progress(&context, 1.0, Some(1.0), "llm_chat done").await;
        Ok(out)
    }

    async fn llm_chat_inner(&self, args: LlmChatToolArgs) -> Result<String, McpError> {
        let LlmChatToolArgs {
            binding_id,
            messages,
            model,
            provider,
            connection_id,
            max_tokens,
            temperature,
            stream,
            prompt_cache_key,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let args = json!({
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "model": model,
            "provider": provider,
            "connection_id": connection_id,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": stream.unwrap_or(false),
            "prompt_cache_key": prompt_cache_key,
        });
        let claim = OfferId::new("llm.chat").expect("valid");
        let resp = self.dispatch_invoke(binding_id, args, Some(claim)).await?;
        serialize_resp(&resp)
    }

    /// Typed invoke for `llm.embed` (returns `InvokeResp` JSON).
    #[tool(description = "Embed texts via bound llm.embed offer (returns InvokeResp)")]
    async fn llm_embed(
        &self,
        Parameters(LlmEmbedArgs {
            binding_id,
            inputs,
            model,
        }): Parameters<LlmEmbedArgs>,
    ) -> Result<String, McpError> {
        let binding_id = parse_binding_id(&binding_id)?;
        let args = json!({ "inputs": inputs, "model": model });
        let claim = OfferId::new("llm.embed").expect("valid");
        let resp = self.dispatch_invoke(binding_id, args, Some(claim)).await?;
        serialize_resp(&resp)
    }

    /// Typed invoke for `llm.preflight` (reachability + capacity fit).
    #[tool(description = "LLM preflight: provider reachability + optional model fit ranks")]
    async fn llm_preflight(
        &self,
        Parameters(LlmPreflightArgs {
            binding_id,
            provider,
            candidates,
        }): Parameters<LlmPreflightArgs>,
    ) -> Result<String, McpError> {
        let binding_id = parse_binding_id(&binding_id)?;
        let cands: Option<Vec<_>> = candidates.map(|cs| {
            cs.into_iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "ram_mb": c.ram_mb,
                        "vram_mb": c.vram_mb.unwrap_or(0),
                    })
                })
                .collect()
        });
        let args = json!({ "provider": provider, "candidates": cands });
        let claim = OfferId::new("llm.preflight").expect("valid");
        let resp = self.dispatch_invoke(binding_id, args, Some(claim)).await?;
        serialize_resp(&resp)
    }

    /// Typed invoke for `llm.ollama.manage` (list/pull/delete).
    #[tool(description = "Manage local Ollama models: list|pull|delete (returns InvokeResp)")]
    async fn ollama_manage(
        &self,
        Parameters(OllamaManageArgs {
            binding_id,
            action,
            model,
        }): Parameters<OllamaManageArgs>,
    ) -> Result<String, McpError> {
        let binding_id = parse_binding_id(&binding_id)?;
        let args = json!({ "action": action, "model": model });
        let claim = OfferId::new("llm.ollama.manage").expect("valid");
        let resp = self.dispatch_invoke(binding_id, args, Some(claim)).await?;
        serialize_resp(&resp)
    }

    /// Typed invoke for `llm.telemetry` (record/list token usage).
    #[tool(description = "Record or list LLM telemetry rows (returns InvokeResp)")]
    async fn llm_telemetry(
        &self,
        Parameters(TelemetryArgs {
            binding_id,
            action,
            record,
            limit,
        }): Parameters<TelemetryArgs>,
    ) -> Result<String, McpError> {
        let binding_id = parse_binding_id(&binding_id)?;
        let args = json!({
            "action": action,
            "record": record,
            "limit": limit,
        });
        let claim = OfferId::new("llm.telemetry").expect("valid");
        let resp = self.dispatch_invoke(binding_id, args, Some(claim)).await?;
        serialize_resp(&resp)
    }

    /// Typed invoke for `sandbox.exec` (returns `InvokeResp` JSON).
    #[tool(description = "Exec via bound sandbox.exec offer (returns InvokeResp)")]
    async fn sandbox_exec(
        &self,
        Parameters(args): Parameters<SandboxExecToolArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<String, McpError> {
        notify_progress(&context, 0.0, Some(1.0), "sandbox_exec start").await;
        let out = self.sandbox_exec_inner(args).await?;
        notify_progress(&context, 1.0, Some(1.0), "sandbox_exec done").await;
        Ok(out)
    }

    async fn sandbox_exec_inner(&self, args: SandboxExecToolArgs) -> Result<String, McpError> {
        let SandboxExecToolArgs {
            binding_id,
            argv,
            cwd,
        } = args;
        let binding_id = parse_binding_id(&binding_id)?;
        let args = json!({ "argv": argv, "cwd": cwd });
        let claim = OfferId::new("sandbox.exec").expect("valid");
        let resp = self.dispatch_invoke(binding_id, args, Some(claim)).await?;
        serialize_resp(&resp)
    }

    #[tool(description = "Read a file in the workspace jail (mode: full|outline|digest)")]
    async fn fs_read(&self, Parameters(args): Parameters<FsReadArgs>) -> Result<String, McpError> {
        self.fs_read_inner(args)
    }

    #[tool(description = "Write a UTF-8 file in the workspace jail")]
    async fn fs_write(
        &self,
        Parameters(args): Parameters<FsWriteArgs>,
    ) -> Result<String, McpError> {
        self.fs_write_inner(args)
    }

    #[tool(description = "Unique substring edit in a jailed file")]
    async fn fs_edit(&self, Parameters(args): Parameters<FsEditArgs>) -> Result<String, McpError> {
        self.fs_edit_inner(args)
    }

    #[tool(description = "Substring grep in a jailed file")]
    async fn fs_grep(&self, Parameters(args): Parameters<FsGrepArgs>) -> Result<String, McpError> {
        self.fs_grep_inner(args)
    }

    #[tool(description = "Run argv in the workspace jail via host shell runner")]
    async fn shell_exec(
        &self,
        Parameters(args): Parameters<ShellExecArgs>,
    ) -> Result<String, McpError> {
        self.shell_exec_inner(args)
    }

    /// Typed invoke for `network.egress.check` (returns `InvokeResp` JSON).
    #[tool(description = "Check URL host against binding egress policy (returns InvokeResp)")]
    async fn egress_check(
        &self,
        Parameters(args): Parameters<EgressCheckArgs>,
    ) -> Result<String, McpError> {
        self.egress_check_inner(args).await
    }

    /// Typed invoke for `network.egress.fetch` (returns `InvokeResp` JSON).
    #[tool(description = "Policy-gated HTTP GET via network.egress.fetch (returns InvokeResp)")]
    async fn egress_fetch(
        &self,
        Parameters(args): Parameters<EgressFetchArgs>,
    ) -> Result<String, McpError> {
        self.egress_fetch_inner(args).await
    }

    /// Typed invoke for `memory.embed`.
    #[tool(description = "Embed texts via bound memory.embed offer (returns InvokeResp)")]
    async fn memory_embed(
        &self,
        Parameters(args): Parameters<MemoryEmbedArgs>,
    ) -> Result<String, McpError> {
        self.memory_embed_inner(args).await
    }

    /// Typed invoke for `memory.scope`.
    #[tool(description = "Hash or list memory scopes via memory.scope (returns InvokeResp)")]
    async fn memory_scope(
        &self,
        Parameters(args): Parameters<MemoryScopeArgs>,
    ) -> Result<String, McpError> {
        self.memory_scope_inner(args).await
    }

    /// Typed invoke for `tools.registry`.
    #[tool(description = "List or get allowlisted tool specs (returns InvokeResp)")]
    async fn tools_registry(
        &self,
        Parameters(args): Parameters<ToolsRegistryArgs>,
    ) -> Result<String, McpError> {
        self.tools_registry_inner(args).await
    }

    /// Typed invoke for `tools.loop`.
    #[tool(description = "Run one tools.loop agent step (returns InvokeResp)")]
    async fn tools_loop(
        &self,
        Parameters(args): Parameters<ToolsLoopArgs>,
    ) -> Result<String, McpError> {
        self.tools_loop_inner(args).await
    }

    /// Typed invoke for `sandbox.jail`.
    #[tool(description = "Inspect sandbox jail root/probe/policy (returns InvokeResp)")]
    async fn sandbox_jail(
        &self,
        Parameters(args): Parameters<SandboxJailArgs>,
    ) -> Result<String, McpError> {
        self.sandbox_jail_inner(args).await
    }

    /// Typed invoke for `memory.index`.
    #[tool(description = "Rebuild memory index from documents (returns InvokeResp)")]
    async fn memory_index(
        &self,
        Parameters(args): Parameters<MemoryIndexArgs>,
    ) -> Result<String, McpError> {
        self.memory_index_inner(args).await
    }

    /// Typed invoke for `memory.search`.
    #[tool(description = "Search the shared memory index (returns InvokeResp)")]
    async fn memory_search(
        &self,
        Parameters(args): Parameters<MemorySearchArgs>,
    ) -> Result<String, McpError> {
        self.memory_search_inner(args).await
    }

    /// Typed invoke for `research.fetch`.
    #[tool(description = "Egress-gated research fetch with sanitize (returns InvokeResp)")]
    async fn research_fetch(
        &self,
        Parameters(args): Parameters<ResearchFetchArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<String, McpError> {
        notify_progress(&context, 0.0, Some(1.0), "research_fetch start").await;
        let out = self.research_fetch_inner(args).await?;
        notify_progress(&context, 1.0, Some(1.0), "research_fetch done").await;
        Ok(out)
    }

    /// Typed invoke for `research.brief`.
    #[tool(description = "Put/get/list research brief artifacts (returns InvokeResp)")]
    async fn research_brief(
        &self,
        Parameters(args): Parameters<ResearchBriefArgs>,
    ) -> Result<String, McpError> {
        self.research_brief_inner(args).await
    }

    /// List locally installed marketplace modules (`sak366-a`).
    #[tool(description = "List locally installed modules (id, version, origin, runtime)")]
    async fn module_list(&self) -> Result<String, McpError> {
        let items = module_registry::list_installed().map_err(|code| {
            McpError::invalid_params(format!("{code}: module list failed"), None)
        })?;
        let modules: Vec<_> = items
            .into_iter()
            .map(|m| {
                json!({
                    "id": m.manifest.id,
                    "version": m.manifest.version,
                    "origin": m.manifest.origin.as_str(),
                    "runtime": m.manifest.runtime.as_str(),
                    "root": m.root.display().to_string(),
                })
            })
            .collect();
        Ok(json!({ "modules": modules }).to_string())
    }

    /// Invoke installed wasm `add` via `ModuleRuntime` cache (`sak366-b`).
    #[tool(description = "Invoke installed wasm module add(a, b); returns {\"sum\": n}")]
    async fn module_invoke(
        &self,
        Parameters(ModuleInvokeArgs { id, a, b }): Parameters<ModuleInvokeArgs>,
    ) -> Result<String, McpError> {
        let installed = module_registry::get_installed(&id, None).map_err(|code| {
            McpError::invalid_params(format!("{code}: module not installed"), None)
        })?;
        let payload = installed.root.join(&installed.manifest.payload);
        let sum = self.modules.invoke_add(&payload, a, b).map_err(|code| {
            McpError::invalid_params(format!("{code}: module invoke failed"), None)
        })?;
        Ok(json!({ "sum": sum, "id": id }).to_string())
    }

    /// Typed invoke for `capacity.probe`.
    #[tool(description = "Probe local hardware capacity (returns InvokeResp)")]
    async fn capacity_probe(
        &self,
        Parameters(args): Parameters<CapacityProbeArgs>,
    ) -> Result<String, McpError> {
        self.capacity_probe_inner(args).await
    }

    /// Typed invoke for `capacity.pressure`.
    #[tool(description = "Sample capacity pressure vs governor budget (returns InvokeResp)")]
    async fn capacity_pressure(
        &self,
        Parameters(args): Parameters<CapacityPressureArgs>,
    ) -> Result<String, McpError> {
        self.capacity_pressure_inner(args).await
    }

    /// Typed invoke for `capacity.fit`.
    #[tool(description = "Rank model candidates by hardware fit (returns InvokeResp)")]
    async fn capacity_fit(
        &self,
        Parameters(args): Parameters<CapacityFitArgs>,
    ) -> Result<String, McpError> {
        self.capacity_fit_inner(args).await
    }

    /// Typed invoke for `compute.node`.
    #[tool(description = "Register/heartbeat/list compute nodes (returns InvokeResp)")]
    async fn compute_node(
        &self,
        Parameters(args): Parameters<ComputeNodeArgs>,
    ) -> Result<String, McpError> {
        self.compute_node_inner(args).await
    }

    /// Typed invoke for `compute.work`.
    #[tool(description = "Enqueue/claim/complete/get compute work units (returns InvokeResp)")]
    async fn compute_work(
        &self,
        Parameters(args): Parameters<ComputeWorkArgs>,
        context: RequestContext<RoleServer>,
    ) -> Result<String, McpError> {
        notify_progress(&context, 0.0, Some(1.0), "compute_work start").await;
        let out = self.compute_work_inner(args).await?;
        notify_progress(&context, 1.0, Some(1.0), "compute_work done").await;
        Ok(out)
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "SwissArmyNoife capability broker v18 (stdio ambient trust — no API key; HTTP uses MCP_HTTP_TOKEN). Tools: ping, broker_health, catalog_list, catalog_get, connections_list, audit_query, provision, bind, unbind, session_bind, invoke, llm_chat, llm_embed, llm_preflight, ollama_manage, llm_telemetry, sandbox_exec, sandbox_jail, fs_read, fs_write, fs_edit, fs_grep, shell_exec, egress_check, egress_fetch, memory_index, memory_embed, memory_scope, memory_search, tools_registry, tools_loop, research_fetch, research_brief, module_list, module_invoke, capacity_probe, capacity_pressure, capacity_fit, compute_node, compute_work. Resources: offer://{id}, binding://{id}."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListResourcesResult, McpError> {
        let store = self.bindings.lock().await;
        Ok(list_resources(&self.catalog, &store))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ReadResourceResult, McpError> {
        let store = self.bindings.lock().await;
        read_resource(&self.catalog, &store, &request.uri)
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
