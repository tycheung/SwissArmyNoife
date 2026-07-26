//! MCP tool argument schemas (kept out of `server.rs` for size).

use rmcp::schemars;
use serde::{Deserialize, Serialize};

use crate::schema_json::{json_value_schema, option_json_value_schema};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CatalogGetArgs {
    /// Offer id (e.g. `llm.chat`).
    pub offer_id: String,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ProvisionArgs {
    /// Offer id to provision.
    pub offer_id: String,
    /// Client idempotency token; replays return the same resource id.
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct BindArgs {
    /// Offer id to bind.
    pub offer_id: String,
    /// Principal identity (ambient local default).
    #[serde(default = "default_principal")]
    pub principal: String,
    /// Frozen policy JSON for the binding TTL.
    #[serde(default)]
    #[schemars(schema_with = "json_value_schema")]
    pub policy: serde_json::Value,
    /// Named policy template (`local-dev`, `strict-egress`, `offline`).
    #[serde(default)]
    pub policy_template: Option<String>,
    /// Client idempotency token; replays return the same binding id.
    #[serde(default)]
    pub idempotency_key: Option<String>,
    /// Binding TTL in seconds (default 3600).
    #[serde(default = "default_ttl_secs")]
    pub ttl_secs: u64,
}

fn default_principal() -> String {
    "local".into()
}

fn default_ttl_secs() -> u64 {
    3600
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UnbindArgs {
    /// Binding id returned by `bind`.
    pub binding_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct InvokeArgs {
    /// Binding id from `bind`.
    pub binding_id: String,
    /// Offer args as JSON.
    #[serde(default)]
    #[schemars(schema_with = "json_value_schema")]
    pub args: serde_json::Value,
    /// Optional offer id claim (must match binding when set).
    #[serde(default)]
    pub offer: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LlmChatToolArgs {
    /// Binding id from `bind` for `llm.chat`.
    pub binding_id: String,
    /// Chat messages (`role` + `content`).
    pub messages: Vec<ChatMessageArg>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Collect provider stream into `chunks` in the `InvokeResp` result.
    #[serde(default)]
    pub stream: Option<bool>,
    /// Optional provider prompt-cache key (passthrough; backends may ignore).
    #[serde(default)]
    pub prompt_cache_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LlmEmbedArgs {
    /// Binding id from `bind` for `llm.embed`.
    pub binding_id: String,
    /// Texts to embed (non-empty).
    pub inputs: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LlmPreflightCandidateArg {
    pub id: String,
    pub ram_mb: u64,
    #[serde(default)]
    pub vram_mb: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LlmPreflightArgs {
    /// Binding id from `bind` for `llm.preflight`.
    pub binding_id: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub candidates: Option<Vec<LlmPreflightCandidateArg>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ChatMessageArg {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SandboxExecToolArgs {
    /// Binding id from `bind` for `sandbox.exec`.
    pub binding_id: String,
    /// Argv; first element is the program.
    pub argv: Vec<String>,
    /// Working directory relative to the sandbox jail (default `.`).
    #[serde(default = "default_cwd")]
    pub cwd: String,
}

pub(crate) fn default_cwd() -> String {
    ".".into()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FsReadArgs {
    pub path: String,
    /// `full` (default), `outline`, or `digest`.
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FsWriteArgs {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FsEditArgs {
    pub path: String,
    pub old: String,
    pub new: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct FsGrepArgs {
    pub path: String,
    pub pattern: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ShellExecArgs {
    pub argv: Vec<String>,
    #[serde(default = "default_cwd")]
    pub cwd: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct EgressFetchArgs {
    /// Binding id from `bind` for `network.egress.fetch`.
    pub binding_id: String,
    /// Absolute URL to fetch (policy-gated).
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct EgressCheckArgs {
    /// Binding id from `bind` for `network.egress.check`.
    pub binding_id: String,
    /// Absolute URL or hostname to check.
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct MemoryDocArg {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct MemoryIndexArgs {
    /// Binding id from `bind` for `memory.index`.
    pub binding_id: String,
    pub documents: Vec<MemoryDocArg>,
    #[serde(default)]
    pub scope_key: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct MemorySearchArgs {
    /// Binding id from `bind` for `memory.search`.
    pub binding_id: String,
    pub query: String,
    #[serde(default)]
    pub k: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ResearchFetchArgs {
    /// Binding id from `bind` for `research.fetch`.
    pub binding_id: String,
    /// Absolute URL to fetch (policy-gated + sanitized).
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ResearchBriefArgs {
    /// Binding id from `bind` for `research.brief`.
    pub binding_id: String,
    /// `put` | `get` | `list`.
    pub action: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ModuleInvokeArgs {
    /// Installed module id (e.g. `community.echo`).
    pub id: String,
    pub a: i32,
    pub b: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CapacityProbeArgs {
    /// Binding id from `bind` for `capacity.probe`.
    pub binding_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CapacityPressureArgs {
    /// Binding id from `bind` for `capacity.pressure`.
    pub binding_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CapacityFitCandidateArg {
    pub id: String,
    pub ram_mb: u64,
    #[serde(default)]
    pub vram_mb: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CapacityFitArgs {
    /// Binding id from `bind` for `capacity.fit`.
    pub binding_id: String,
    pub candidates: Vec<CapacityFitCandidateArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ComputeNodeArgs {
    /// Binding id from `bind` for `compute.node`.
    pub binding_id: String,
    /// `register` | `heartbeat` | `list`.
    pub action: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub caps: Option<Vec<String>>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub stale_secs: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ComputeWorkArgs {
    /// Binding id from `bind` for `compute.work`.
    pub binding_id: String,
    /// `enqueue` | `claim` | `complete` | `get` | `list` | `requeue`.
    pub action: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "option_json_value_schema")]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "option_json_value_schema")]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub stage_name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SessionBindArgs {
    /// Offer ids to bind together (e.g. `llm.chat`, `sandbox.exec`).
    pub offer_ids: Vec<String>,
    #[serde(default)]
    pub principal: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "option_json_value_schema")]
    pub policy: Option<serde_json::Value>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct OllamaManageArgs {
    /// Binding id from `bind` for `llm.ollama.manage`.
    pub binding_id: String,
    /// `list` | `pull` | `delete`.
    pub action: String,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub(crate) struct TelemetryRecordArg {
    pub provider: String,
    pub binding_source: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub connection_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct TelemetryArgs {
    /// Binding id from `bind` for `llm.telemetry`.
    pub binding_id: String,
    /// `record` | `list`.
    pub action: String,
    #[serde(default)]
    pub record: Option<TelemetryRecordArg>,
    #[serde(default)]
    pub limit: Option<usize>,
}
