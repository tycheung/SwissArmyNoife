//! Cursor-safe JSON Schema helpers for free-form JSON tool fields (`mcp-cursor-schemas`).

use rmcp::schemars::{json_schema, Schema, SchemaGenerator};

/// JSON object schema for `serde_json::Value` (Cursor rejects typeless props).
pub fn json_value_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": "object" })
}

/// Optional JSON object (`null` or object).
pub fn option_json_value_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": ["object", "null"] })
}
