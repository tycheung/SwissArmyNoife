//! `llm.telemetry` — token / binding_source records (`sak140`).

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

/// One LLM invoke telemetry row (no secrets).
#[derive(Clone, Debug, PartialEq, serde::Serialize, Deserialize)]
pub struct TelemetryRecord {
    pub provider: String,
    pub binding_source: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
}

/// Process-local telemetry ring for `llm.telemetry`.
pub struct LlmTelemetryOffer {
    entry: CatalogEntry,
    records: Mutex<Vec<TelemetryRecord>>,
    cap: usize,
}

impl LlmTelemetryOffer {
    /// # Errors
    /// Invalid catalog id.
    pub fn new() -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("llm.telemetry", "0.1.0")?,
            records: Mutex::new(Vec::new()),
            cap: 256,
        })
    }
}

impl Offer for LlmTelemetryOffer {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-llm.telemetry".into())
    }

    async fn bind(&self, _binding_id: BindingId, _params: Value) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run(self, &req.args) {
            Ok(v) => InvokeResp::ok(invoke_id, v),
            Err((code, message)) => InvokeResp::Error {
                invoke_id: Some(invoke_id),
                code,
                message,
            },
        }
    }

    async fn unbind(&self, _binding_id: BindingId) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn health(&self) -> Result<(), ErrorCode> {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct TelemetryArgs {
    action: String,
    #[serde(default)]
    record: Option<TelemetryRecord>,
    #[serde(default)]
    limit: Option<usize>,
}

fn run(offer: &LlmTelemetryOffer, args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: TelemetryArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("telemetry args: {e}")))?;
    match parsed.action.as_str() {
        "record" => {
            let rec = parsed.record.ok_or_else(|| {
                (
                    ErrorCode::SchemaInvalid,
                    "record requires record object".into(),
                )
            })?;
            if rec.provider.is_empty() || rec.binding_source.is_empty() {
                return Err((
                    ErrorCode::SchemaInvalid,
                    "provider and binding_source required".into(),
                ));
            }
            let mut guard = offer
                .records
                .lock()
                .map_err(|_| (ErrorCode::SchemaInvalid, "lock".into()))?;
            guard.push(rec);
            while guard.len() > offer.cap {
                guard.remove(0);
            }
            Ok(json!({ "recorded": true, "count": guard.len() }))
        }
        "list" => {
            let limit = parsed.limit.unwrap_or(50).min(256);
            let guard = offer
                .records
                .lock()
                .map_err(|_| (ErrorCode::SchemaInvalid, "lock".into()))?;
            let rows: Vec<_> = guard.iter().rev().take(limit).cloned().collect();
            Ok(json!({ "records": rows }))
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("action must be record|list, got {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn record_and_list() {
        let offer = LlmTelemetryOffer::new().unwrap();
        let bind = BindingId::from_uuid(Uuid::nil());
        let resp = offer
            .invoke(InvokeReq {
                binding_id: bind,
                args: json!({
                    "action": "record",
                    "record": {
                        "provider": "echo",
                        "binding_source": "local",
                        "prompt_tokens": 3,
                        "completion_tokens": 2,
                        "model": "fixture"
                    }
                }),
                invoke_id: None,
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => assert_eq!(result["recorded"], true),
            other => panic!("{other:?}"),
        }
        let listed = offer
            .invoke(InvokeReq {
                binding_id: bind,
                args: json!({ "action": "list", "limit": 10 }),
                invoke_id: None,
                offer: None,
            })
            .await;
        match listed {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["records"][0]["provider"], "echo");
                assert_eq!(result["records"][0]["prompt_tokens"], 3);
            }
            other => panic!("{other:?}"),
        }
    }
}
