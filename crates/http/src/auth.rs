//! HTTP admin bearer auth (`sak541-a`) — mirrors mcp-http posture.

use std::sync::Arc;

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use control::ApiKeyStore;

/// Env: static bearer expected by admin HTTP (`MCP_HTTP_TOKEN`).
pub const HTTP_TOKEN_ENV: &str = "MCP_HTTP_TOKEN";

/// Env: when `1`, allow unauthenticated loopback/tests (`MCP_HTTP_ALLOW_INSECURE`).
pub const HTTP_ALLOW_INSECURE_ENV: &str = "MCP_HTTP_ALLOW_INSECURE";

/// Resolve expected token: `None` means insecure / no auth required.
#[must_use]
pub fn token_from_env() -> Option<String> {
    if std::env::var(HTTP_ALLOW_INSECURE_ENV).is_ok_and(|v| v == "1") {
        return None;
    }
    std::env::var(HTTP_TOKEN_ENV).ok().filter(|s| !s.is_empty())
}

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

/// Axum middleware: `MCP_HTTP_TOKEN` bearer or minted API key.
pub async fn auth_middleware(
    expected_token: Option<String>,
    api_keys: Arc<ApiKeyStore>,
    req: Request,
    next: Next,
) -> Response {
    let header = req
        .headers()
        .get(header::AUTHORIZATION)
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
        assert!(bearer_authorized(Some("tok"), &keys, Some("Bearer tok")));
        assert!(!bearer_authorized(Some("tok"), &keys, Some("Bearer wrong")));
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

    #[test]
    fn token_from_env_respects_insecure_flag() {
        std::env::set_var(HTTP_ALLOW_INSECURE_ENV, "1");
        std::env::set_var(HTTP_TOKEN_ENV, "secret");
        assert_eq!(token_from_env(), None);
        std::env::remove_var(HTTP_ALLOW_INSECURE_ENV);
        std::env::remove_var(HTTP_TOKEN_ENV);
    }
}
