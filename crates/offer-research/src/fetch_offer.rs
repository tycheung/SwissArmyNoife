//! `research.fetch` — egress-gated GET + sanitize.

use std::sync::Mutex;

use control::{CatalogEntry, Offer};
use offer_egress::{EgressPolicy, HttpGet, ReqwestGet};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp};

use crate::fetch::research_fetch;

/// First-party research fetch offer.
pub struct ResearchFetchOffer<H> {
    entry: CatalogEntry,
    policy: Mutex<EgressPolicy>,
    principal: Mutex<String>,
    http: H,
}

impl ResearchFetchOffer<ReqwestGet> {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when catalog id is empty.
    pub fn new() -> Result<Self, ErrorCode> {
        Self::with_http(ReqwestGet::new())
    }
}

impl<H> ResearchFetchOffer<H> {
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] when catalog id is empty.
    pub fn with_http(http: H) -> Result<Self, ErrorCode> {
        Ok(Self {
            entry: CatalogEntry::new("research.fetch", "0.1.0")?,
            policy: Mutex::new(EgressPolicy::default()),
            principal: Mutex::new("local".into()),
            http,
        })
    }
}

impl<H: HttpGet + Send + Sync> Offer for ResearchFetchOffer<H> {
    fn catalog_entry(&self) -> &CatalogEntry {
        &self.entry
    }

    async fn provision(&self, _params: Value) -> Result<String, ErrorCode> {
        Ok("res-research.fetch".into())
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
        match run(&self.policy, &self.principal, &self.http, &req.args).await {
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

async fn run<H: HttpGet>(
    policy: &Mutex<EgressPolicy>,
    frozen_principal: &Mutex<String>,
    http: &H,
    args: &Value,
) -> Result<Value, (ErrorCode, String)> {
    let parsed: FetchArgs = serde_json::from_value(args.clone()).map_err(|e| {
        (
            ErrorCode::SchemaInvalid,
            format!("research.fetch args: {e}"),
        )
    })?;
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
    let body = research_fetch(&policy_snap, principal, &parsed.url, http)
        .await
        .map_err(|code| (code, format!("{code}: research fetch failed")))?;
    Ok(json!({
        "host": body.host,
        "status": body.status,
        "raw_bytes": body.raw_bytes,
        "text": body.text,
        "url": parsed.url,
        "principal": principal,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::InvokeId;

    struct StubHttp;

    impl HttpGet for StubHttp {
        async fn get(&self, _url: &str) -> Result<(u16, Vec<u8>), ErrorCode> {
            Ok((200, b"<h1>Title</h1><script>bad</script>".to_vec()))
        }
    }

    #[tokio::test]
    async fn fetch_offer_ok() {
        let offer = ResearchFetchOffer::with_http(StubHttp).expect("offer");
        offer
            .bind(
                BindingId::new(),
                json!({
                    "egress": {
                        "allow_hosts": ["example.com"],
                        "allow_principals": ["local"],
                        "max_response_bytes": 1024
                    }
                }),
            )
            .await
            .expect("bind");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: json!({ "url": "https://example.com/x" }),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                assert_eq!(result["text"], "Title");
                assert!(!result["text"].as_str().unwrap().contains("script"));
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }
}
