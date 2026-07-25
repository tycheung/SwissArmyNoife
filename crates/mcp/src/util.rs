//! Shared MCP helpers (binding parse, response serialize).

use std::time::{SystemTime, UNIX_EPOCH};

use rmcp::ErrorData as McpError;
use types::{BindingId, InvokeResp};
use uuid::Uuid;

pub(crate) fn parse_binding_id(raw: &str) -> Result<BindingId, McpError> {
    let uuid = Uuid::parse_str(raw)
        .map_err(|_| McpError::invalid_params("schema.invalid: bad binding_id", None))?;
    Ok(BindingId::from_uuid(uuid))
}

pub(crate) fn expires_unix(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

pub(crate) fn serialize_resp(resp: &InvokeResp) -> Result<String, McpError> {
    serde_json::to_string(resp)
        .map_err(|e| McpError::internal_error(format!("serialize InvokeResp: {e}"), None))
}
