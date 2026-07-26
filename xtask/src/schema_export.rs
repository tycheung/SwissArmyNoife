//! Schema export via schemars (`sak530-a`) — emit MCP tool input schemas.
//!
//! Delegates to `cargo run -p cli -- schema tools` (same document as
//! `mcp::tool_input_schemas()`). Stub dry-run (`sak111-i`) is retired.

use serde_json::Value;
use std::process::Command;

/// Canonical tool names aligned with `mcp` schema dump drift gate (`sak111-b`).
pub const CANONICAL_TOOL_NAMES: &[&str] = &[
    "ping",
    "broker_health",
    "catalog_list",
    "catalog_get",
    "provision",
    "bind",
    "unbind",
    "session_bind",
    "invoke",
    "llm_chat",
    "llm_preflight",
    "sandbox_exec",
    "fs_read",
    "memory_search",
    "research_fetch",
    "compute_work",
    "module_list",
    "module_invoke",
];

/// Emit the live schemars tool-input document (pretty JSON).
///
/// # Errors
/// When `cli` schema tools fails to spawn, exits non-zero, or returns invalid UTF-8.
pub fn emit_document() -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "cli", "--", "schema", "tools"])
        .output()
        .map_err(|e| format!("spawn cargo run -p cli: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cli schema tools failed (status {:?}): {stderr}",
            output.status.code()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("schema tools stdout utf-8: {e}"))
}

/// Validate emitted document matches hand schemars schemas for canonical tools.
///
/// # Errors
/// When emit fails, JSON is invalid, or a canonical tool / bind field is missing.
pub fn check_schemars() -> Result<(), String> {
    let raw = emit_document()?;
    let doc: Value =
        serde_json::from_str(&raw).map_err(|e| format!("schema tools JSON parse: {e}"))?;
    let tools = doc
        .get("tools")
        .and_then(|t| t.as_object())
        .ok_or_else(|| "missing tools object".to_string())?;
    for name in CANONICAL_TOOL_NAMES {
        let Some(schema) = tools.get(*name) else {
            return Err(format!("missing schema for {name}"));
        };
        if schema.get("type").and_then(|t| t.as_str()) != Some("object") {
            return Err(format!("{name}: expected type=object"));
        }
        if schema.get("x-sak-codegen").and_then(|v| v.as_str()) == Some("stub") {
            return Err(format!("{name}: still a stub schema"));
        }
    }
    let bind = tools
        .get("bind")
        .ok_or_else(|| "missing bind".to_string())?;
    let props = bind
        .get("properties")
        .and_then(|p| p.as_object())
        .ok_or_else(|| "bind missing properties".to_string())?;
    if !props.contains_key("idempotency_key") {
        return Err("bind missing idempotency_key (hand schema drift)".into());
    }
    if !props.contains_key("policy_template") {
        return Err("bind missing policy_template (hand schema drift)".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_includes_bind_and_module_invoke() {
        assert!(CANONICAL_TOOL_NAMES.contains(&"bind"));
        assert!(CANONICAL_TOOL_NAMES.contains(&"module_invoke"));
        assert_eq!(CANONICAL_TOOL_NAMES.len(), 18);
    }
}
