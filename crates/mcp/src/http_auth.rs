//! Streamable HTTP bearer auth (`sak059-c`).

use std::sync::Arc;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use control::ApiKeyStore;
use http::StatusCode;

/// Pure bearer check for unit tests and middleware.
#[must_use]
pub fn bearer_authorized(expected: Option<&str>, keys: &ApiKeyStore, header: Option<&str>) -> bool {
    if expected.is_none() {
        return true;
    }
    let Some(header) = header else {
        return false;
    };
    let Some(token) = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
    else {
        return false;
    };
    if expected.is_some_and(|want| token == want) {
        return true;
    }
    keys.verify(token).is_ok()
}

/// Axum middleware: `MCP_HTTP_TOKEN` bearer or `sk_live_…` API key.
pub async fn auth_middleware(
    expected_token: Option<String>,
    api_keys: Arc<ApiKeyStore>,
    req: Request,
    next: Next,
) -> Response {
    let header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if !bearer_authorized(expected_token.as_deref(), api_keys.as_ref(), header) {
        return (StatusCode::UNAUTHORIZED, "missing or invalid bearer token").into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_token_matches() {
        let keys = ApiKeyStore::new();
        assert!(bearer_authorized(Some("tok"), &keys, Some("Bearer tok"),));
        assert!(!bearer_authorized(Some("tok"), &keys, Some("Bearer wrong"),));
    }

    #[test]
    fn api_key_verifies_when_token_mismatch() {
        let keys = ApiKeyStore::new();
        let (_info, secret) = keys.mint("alice").unwrap();
        assert!(bearer_authorized(
            Some("mcp-static-token"),
            &keys,
            Some(&format!("Bearer {secret}")),
        ));
    }

    #[test]
    fn insecure_mode_allows_without_header() {
        let keys = ApiKeyStore::new();
        assert!(bearer_authorized(None, &keys, None));
    }
}
