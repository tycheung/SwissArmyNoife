//! Policy-gated HTTP GET with response byte caps.

use types::ErrorCode;

use crate::body_cap::collect_capped;
use crate::policy::EgressPolicy;

/// Successful guarded fetch body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardedBody {
    pub host: String,
    pub bytes: Vec<u8>,
    pub status: u16,
}

/// Minimal GET surface (real HTTP or test doubles).
pub trait HttpGet: Send + Sync {
    /// Fetch response status + body bytes.
    fn get(
        &self,
        url: &str,
    ) -> impl std::future::Future<Output = Result<(u16, Vec<u8>), ErrorCode>> + Send;
}

/// `reqwest`-backed GET.
#[derive(Clone, Debug, Default)]
pub struct ReqwestGet {
    client: reqwest::Client,
}

impl ReqwestGet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl HttpGet for ReqwestGet {
    async fn get(&self, url: &str) -> Result<(u16, Vec<u8>), ErrorCode> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?;
        let status = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|_| ErrorCode::ProviderUnreachable)?
            .to_vec();
        Ok((status, bytes))
    }
}

/// Check egress policy then GET, enforcing `max_response_bytes`.
///
/// # Errors
/// Policy / egress / budget / provider codes.
pub async fn guarded_get(
    policy: &EgressPolicy,
    principal: &str,
    url: &str,
    http: &impl HttpGet,
) -> Result<GuardedBody, ErrorCode> {
    let host = policy.check(principal, url)?;
    let (status, raw) = http.get(url).await?;
    let bytes = collect_capped(&policy.max_response_bytes, [raw.as_slice()])?;
    Ok(GuardedBody {
        host,
        bytes,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EgressPolicy;
    use serde_json::json;
    use std::sync::Mutex;

    struct StubHttp {
        bodies: Mutex<Vec<(u16, Vec<u8>)>>,
    }

    impl HttpGet for StubHttp {
        async fn get(&self, _url: &str) -> Result<(u16, Vec<u8>), ErrorCode> {
            let mut g = self.bodies.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
            g.pop().ok_or(ErrorCode::ProviderUnreachable)
        }
    }

    #[tokio::test]
    async fn allow_and_byte_cap() {
        let policy = EgressPolicy::from_policy(&json!({
            "egress": {
                "allow_hosts": ["api.example.com"],
                "allow_principals": ["local"],
                "max_response_bytes": 4
            }
        }));
        let http = StubHttp {
            bodies: Mutex::new(vec![(200, b"ok!!".to_vec())]),
        };
        let body = guarded_get(&policy, "local", "https://api.example.com/x", &http)
            .await
            .expect("ok");
        assert_eq!(body.bytes, b"ok!!");
        assert_eq!(body.status, 200);

        let http2 = StubHttp {
            bodies: Mutex::new(vec![(200, b"too-big".to_vec())]),
        };
        assert_eq!(
            guarded_get(&policy, "local", "https://api.example.com/x", &http2)
                .await
                .expect_err("cap"),
            ErrorCode::BudgetExhausted
        );

        assert_eq!(
            guarded_get(&policy, "local", "https://evil.com/", &http2)
                .await
                .expect_err("deny"),
            ErrorCode::EgressDenied
        );
    }
}
