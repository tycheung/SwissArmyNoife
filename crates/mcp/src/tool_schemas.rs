//! Dump MCP tool input JSON Schemas (`sak111-a`) — Cursor-facing contract.

use rmcp::handler::server::common::schema_for_type;
use serde_json::{json, Map, Value};

use crate::tool_args::{
    AuditQueryArgs, BindArgs, CapacityFitArgs, CapacityPressureArgs, CapacityProbeArgs,
    CatalogGetArgs, ComputeNodeArgs, ComputeWorkArgs, EgressCheckArgs, EgressFetchArgs, FsEditArgs,
    FsGrepArgs, FsReadArgs, FsWriteArgs, InvokeArgs, LlmChatToolArgs, LlmEmbedArgs,
    LlmPreflightArgs, MemoryEmbedArgs, MemoryIndexArgs, MemoryScopeArgs, MemorySearchArgs,
    ModuleInvokeArgs, OllamaManageArgs, ProvisionArgs, RateLimitStatusArgs, ResearchBriefArgs,
    ResearchFetchArgs, SandboxExecToolArgs, SandboxJailArgs, SessionBindArgs, ShellExecArgs,
    TelemetryArgs, ToolsLoopArgs, ToolsRegistryArgs, UnbindArgs,
};

/// Map of tool name → input JSON Schema object (draft 2020-12 via schemars).
#[must_use]
pub fn tool_input_schemas() -> Value {
    let mut m = Map::new();
    insert::<CatalogGetArgs>(&mut m, "catalog_get");
    insert::<AuditQueryArgs>(&mut m, "audit_query");
    insert::<RateLimitStatusArgs>(&mut m, "rate_limit_status");
    insert::<ProvisionArgs>(&mut m, "provision");
    insert::<BindArgs>(&mut m, "bind");
    insert::<UnbindArgs>(&mut m, "unbind");
    insert::<SessionBindArgs>(&mut m, "session_bind");
    insert::<InvokeArgs>(&mut m, "invoke");
    insert::<LlmChatToolArgs>(&mut m, "llm_chat");
    insert::<LlmEmbedArgs>(&mut m, "llm_embed");
    insert::<LlmPreflightArgs>(&mut m, "llm_preflight");
    insert::<OllamaManageArgs>(&mut m, "ollama_manage");
    insert::<TelemetryArgs>(&mut m, "llm_telemetry");
    insert::<SandboxExecToolArgs>(&mut m, "sandbox_exec");
    insert::<SandboxJailArgs>(&mut m, "sandbox_jail");
    insert::<FsReadArgs>(&mut m, "fs_read");
    insert::<FsWriteArgs>(&mut m, "fs_write");
    insert::<FsEditArgs>(&mut m, "fs_edit");
    insert::<FsGrepArgs>(&mut m, "fs_grep");
    insert::<ShellExecArgs>(&mut m, "shell_exec");
    insert::<EgressCheckArgs>(&mut m, "egress_check");
    insert::<EgressFetchArgs>(&mut m, "egress_fetch");
    insert::<MemoryIndexArgs>(&mut m, "memory_index");
    insert::<MemoryEmbedArgs>(&mut m, "memory_embed");
    insert::<MemoryScopeArgs>(&mut m, "memory_scope");
    insert::<MemorySearchArgs>(&mut m, "memory_search");
    insert::<ToolsRegistryArgs>(&mut m, "tools_registry");
    insert::<ToolsLoopArgs>(&mut m, "tools_loop");
    insert::<ResearchFetchArgs>(&mut m, "research_fetch");
    insert::<ResearchBriefArgs>(&mut m, "research_brief");
    insert::<ModuleInvokeArgs>(&mut m, "module_invoke");
    insert::<CapacityProbeArgs>(&mut m, "capacity_probe");
    insert::<CapacityPressureArgs>(&mut m, "capacity_pressure");
    insert::<CapacityFitArgs>(&mut m, "capacity_fit");
    insert::<ComputeNodeArgs>(&mut m, "compute_node");
    insert::<ComputeWorkArgs>(&mut m, "compute_work");
    // Zero-arg tools still appear in Cursor list; document empty object schemas.
    m.insert(
        "ping".into(),
        json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );
    m.insert(
        "broker_health".into(),
        json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );
    m.insert(
        "catalog_list".into(),
        json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );
    m.insert(
        "connections_list".into(),
        json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );
    m.insert(
        "module_list".into(),
        json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );
    json!({ "tools": m })
}

fn insert<T: rmcp::schemars::JsonSchema + 'static>(map: &mut Map<String, Value>, name: &str) {
    let schema = schema_for_type::<T>();
    map.insert(name.to_owned(), Value::Object((*schema).clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_policy_has_object_type() {
        let doc = tool_input_schemas();
        let policy = &doc["tools"]["bind"]["properties"]["policy"];
        assert_eq!(policy["type"], "object", "{policy}");
        assert!(
            doc["tools"]["llm_embed"].is_object(),
            "sak523-b llm_embed schema"
        );
        assert!(
            doc["tools"]["memory_embed"].is_object(),
            "sak524-a memory_embed schema"
        );
        assert!(
            doc["tools"]["memory_scope"].is_object(),
            "sak524-c memory_scope schema"
        );
        assert!(
            doc["tools"]["tools_registry"].is_object(),
            "sak525-b tools_registry schema"
        );
        assert!(
            doc["tools"]["tools_loop"].is_object(),
            "sak525-d tools_loop schema"
        );
        assert!(
            doc["tools"]["sandbox_jail"].is_object(),
            "sak526-b sandbox_jail schema"
        );
        assert!(
            doc["tools"]["connections_list"].is_object(),
            "sak527-d connections_list schema"
        );
        assert!(
            doc["tools"]["audit_query"].is_object(),
            "sak528-b audit_query schema"
        );
        assert!(
            doc["tools"]["rate_limit_status"].is_object(),
            "sak528-c rate_limit_status schema"
        );
    }
}
