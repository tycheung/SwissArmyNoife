//! Research fetch via egress-gated HTTP (`sak240`).

use offer_egress::{guarded_get, EgressPolicy, HttpGet};
use types::ErrorCode;

use crate::sanitize::sanitize_untrusted;

/// Successful research fetch after sanitize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchBody {
    pub host: String,
    pub status: u16,
    pub raw_bytes: usize,
    pub text: String,
}

/// Policy-gated GET then sanitize body text.
///
/// # Errors
/// Propagates egress / provider / budget codes from [`guarded_get`].
pub async fn research_fetch(
    policy: &EgressPolicy,
    principal: &str,
    url: &str,
    http: &impl HttpGet,
) -> Result<ResearchBody, ErrorCode> {
    let body = guarded_get(policy, principal, url, http).await?;
    let text = sanitize_untrusted(&String::from_utf8_lossy(&body.bytes));
    Ok(ResearchBody {
        host: body.host,
        status: body.status,
        raw_bytes: body.bytes.len(),
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    struct StubHttp {
        body: Mutex<Vec<u8>>,
    }

    impl HttpGet for StubHttp {
        async fn get(&self, _url: &str) -> Result<(u16, Vec<u8>), ErrorCode> {
            Ok((200, self.body.lock().expect("lock").clone()))
        }
    }

    #[tokio::test]
    async fn fetch_sanitizes() {
        let policy = EgressPolicy::from_policy(&json!({
            "egress": {
                "allow_hosts": ["docs.example.com"],
                "allow_principals": ["local"],
                "max_response_bytes": 4096
            }
        }));
        let http = StubHttp {
            body: Mutex::new(b"<p>Safe</p><script>x</script>".to_vec()),
        };
        let out = research_fetch(&policy, "local", "https://docs.example.com/a", &http)
            .await
            .expect("ok");
        assert_eq!(out.text, "Safe");
        assert_eq!(out.status, 200);
    }
}
