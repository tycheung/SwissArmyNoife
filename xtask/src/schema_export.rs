//! Schema export dry-run (`sak111-h`) — stub JSON Schema walk until schemars codegen.
//!
//! Full `types` → JSON Schema codegen remains deferred (`docs/mcp-schema-codegen.md`).
//! This module walks the canonical MCP tool name list and emits / checks stub schemas
//! so CI can exercise the pipeline without breaking hand drift tests.

use std::collections::BTreeMap;
use std::fmt::Write as _;

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

/// Minimal JSON Schema stub for a tool input object.
#[must_use]
pub fn stub_schema_for(tool: &str) -> String {
    format!(
        r#"{{"type":"object","properties":{{}},"additionalProperties":false,"x-sak-codegen":"stub","x-sak-tool":"{tool}"}}"#
    )
}

/// Build tool → stub schema map (sorted).
#[must_use]
pub fn stub_schema_map() -> BTreeMap<&'static str, String> {
    CANONICAL_TOOL_NAMES
        .iter()
        .map(|name| (*name, stub_schema_for(name)))
        .collect()
}

/// Validate stub map: every canonical name present, stub parses as object-ish JSON.
pub fn check_stubs() -> Result<(), String> {
    let map = stub_schema_map();
    if map.len() != CANONICAL_TOOL_NAMES.len() {
        return Err(format!(
            "stub count {} != canonical {}",
            map.len(),
            CANONICAL_TOOL_NAMES.len()
        ));
    }
    for name in CANONICAL_TOOL_NAMES {
        let Some(stub) = map.get(name) else {
            return Err(format!("missing stub for {name}"));
        };
        if !stub.contains("\"type\":\"object\"") {
            return Err(format!("{name}: stub missing type=object"));
        }
        if !stub.contains("\"x-sak-codegen\":\"stub\"") {
            return Err(format!("{name}: stub missing x-sak-codegen"));
        }
        if !stub.contains(&format!("\"x-sak-tool\":\"{name}\"")) {
            return Err(format!("{name}: stub missing x-sak-tool"));
        }
    }
    Ok(())
}

/// Pretty-print stub document for `schema export` (stdout).
#[must_use]
pub fn emit_stubs_document() -> String {
    let map = stub_schema_map();
    let mut out = String::from("{\n  \"tools\": {\n");
    let mut first = true;
    for (name, stub) in &map {
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let _ = write!(out, "    \"{name}\": {stub}");
    }
    out.push_str("\n  },\n  \"x-sak-codegen\": \"stub-dry-run\",\n");
    out.push_str(
        "  \"note\": \"full types→JSON Schema deferred; see docs/mcp-schema-codegen.md\"\n}\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_stubs_ok() {
        check_stubs().expect("stubs");
    }

    #[test]
    fn emit_includes_bind_and_module_invoke() {
        let doc = emit_stubs_document();
        assert!(doc.contains("\"bind\""));
        assert!(doc.contains("\"module_invoke\""));
        assert!(doc.contains("stub-dry-run"));
    }
}
