//! Invoke request/response wire types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{BindingId, ErrorCode, OfferId};

/// Correlation id for a single invoke (tracing / audit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct InvokeId(Uuid);

impl InvokeId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for InvokeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for InvokeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `invoke` verb input: binding + JSON args (offer optional when binding implies it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvokeReq {
    pub binding_id: BindingId,
    /// Offer args as JSON (schema validated per offer at the control plane).
    pub args: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoke_id: Option<InvokeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer: Option<OfferId>,
}

/// `invoke` verb output: success payload or stable error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum InvokeResp {
    Ok {
        invoke_id: InvokeId,
        result: Value,
    },
    Error {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        invoke_id: Option<InvokeId>,
        code: ErrorCode,
        message: String,
    },
}

impl InvokeResp {
    #[must_use]
    pub fn ok(invoke_id: InvokeId, result: Value) -> Self {
        Self::Ok { invoke_id, result }
    }

    #[must_use]
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Error {
            invoke_id: None,
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn invoke_req_roundtrip() {
        let req = InvokeReq {
            binding_id: BindingId::from_uuid(Uuid::nil()),
            args: json!({"prompt": "hi"}),
            invoke_id: Some(InvokeId::from_uuid(Uuid::nil())),
            offer: Some(OfferId::new("llm.chat").expect("valid")),
        };
        let json = serde_json::to_string(&req).expect("serialize");
        let back: InvokeReq = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, req);
    }

    #[test]
    fn invoke_req_rejects_unknown_fields() {
        let raw = r#"{
            "binding_id": "00000000-0000-0000-0000-000000000000",
            "args": {},
            "extra_critical": true
        }"#;
        let err = serde_json::from_str::<InvokeReq>(raw).expect_err("unknown field");
        let msg = err.to_string();
        assert!(
            msg.contains("extra_critical") || msg.contains("unknown field"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn invoke_resp_ok_and_error_roundtrip() {
        let ok = InvokeResp::ok(InvokeId::from_uuid(Uuid::nil()), json!({"text": "yo"}));
        let ok_json = serde_json::to_value(&ok).expect("serialize ok");
        assert_eq!(ok_json["status"], "ok");
        let ok_back: InvokeResp = serde_json::from_value(ok_json).expect("deserialize ok");
        assert_eq!(ok_back, ok);

        let err = InvokeResp::error(ErrorCode::BindingExpired, "ttl elapsed");
        let err_json = serde_json::to_value(&err).expect("serialize err");
        assert_eq!(err_json["status"], "error");
        assert_eq!(err_json["code"], "binding.expired");
        let err_back: InvokeResp = serde_json::from_value(err_json).expect("deserialize err");
        assert_eq!(err_back, err);
    }
}
