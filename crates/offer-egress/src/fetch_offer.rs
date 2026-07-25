//! `network.egress.fetch` offer: policy check + guarded HTTP GET.

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::fetch::{guarded_get, HttpGet, ReqwestGet};
use crate::policy::EgressPolicy;

/// First-party egress fetch offer.
pub struct EgressFetchOffer<H> {
    entry: CatalogEntry,
    policy: Mutex<EgressPolicy>,
    principal: Mutex<String>,
    http: H,
}

impl EgressFetchOffer<ReqwestGet> {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when catalog id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Self::with_http(ReqwestGet::new())
    }
}

impl<H> EgressFetchOffer<H> {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when catalog id is empty.
    pub fn with_http(http: H) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("network.egress.fetch", "0.1.0")?,
            policy: Mutex::new(EgressPolicy::default()),
            principal: Mutex::new("local".into()),
            http,
        })
    }
}

impl<H: HttpGet + Send + Sync> Offer for EgressFetchOffer<H> {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-network.egress.fetch".into())
    }

    async fn bind(&self, _binding_id: BindingId, params: Value) -> Result<(), ErrorCode> {
        let mut policy = self.policy.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        *policy = EgressPolicy::from_policy(&params);
        if let Some(p) = params.get("principal").and_then(Value::as_str) {
            let mut principal = self
                .principal
                .lock()
                .map_err(|_| ErrorCode::SchemaInvalid)?;
            p.clone_into(&mut *principal);
        }
        Ok(())
    }

    async fn invoke(&self, req: InvokeReq) -> InvokeResp {
        let invoke_id = req.invoke_id.unwrap_or_default();
        match run_fetch(&self.policy, &self.principal, &self.http, &req.args).await {
            Ok(result) => InvokeResp::ok(invoke_id, result),
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
struct FetchArgs {
    url: String,
    #[serde(default)]
    principal: Option<String>,
}

async fn run_fetch<H: HttpGet>(
    policy: &Mutex<EgressPolicy>,
    frozen_principal: &Mutex<String>,
    http: &H,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: FetchArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("fetch args: {e}")))?;
    let (policy_snap, principal_default) = {
        let policy = policy
            .lock()
            .map_err(|_| (ErrorCode::SchemaInvalid, "policy lock".into()))?;
        let default_principal = frozen_principal
            .lock()
            .map_err(|_| (ErrorCode::SchemaInvalid, "principal lock".into()))?;
        (policy.clone(), default_principal.clone())
    };
    let principal = parsed
        .principal
        .as_deref()
        .unwrap_or(principal_default.as_str());
    let body = guarded_get(&policy_snap, principal, &parsed.url, http)
        .await
        .map_err(|code| (code, format!("{code}: egress fetch failed")))?;
    let text = String::from_utf8_lossy(&body.bytes).into_owned();
    Ok(json!({
        "host": body.host,
        "status": body.status,
        "bytes": body.bytes.len(),
        "body_text": text,
        "url": parsed.url,
        "principal": principal,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::HttpGet;
    use types::InvokeId;

    struct StubHttp {
        body: Vec<u8>,
    }

    impl HttpGet for StubHttp {
        async fn get(&self, _url: &str) -> Result<(u16, Vec<u8>), ErrorCode> {
            Ok((200, self.body.clone()))
        }
    }

    #[tokio::test]
    async fn fetch_allow() {
        let offer = EgressFetchOffer::with_http(StubHttp {
            body: b"hello".to_vec(),
        })
        .expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({
                    "egress": {
                        "allow_hosts": ["api.example.com"],
                        "allow_principals": ["local"],
                        "max_response_bytes": 64
                    }
                }),
            )
            .await
            .expect("bind");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "url": "https://api.example.com/x" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["body_text"], "hello");
                assert_eq!(result["status"], 200);
            }
            other @ InvokeResp::Error { .. } => panic!("expected ok, got {other:?}"),
        }
    }
}
