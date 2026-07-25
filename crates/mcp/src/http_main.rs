//! Streamable HTTP MCP entrypoint (`sak110` / `sak113` / `sak059-c`).
//!
//! Requires `MCP_HTTP_TOKEN` unless `MCP_HTTP_ALLOW_INSECURE=1` (tests / loopback only).

use std::net::SocketAddr;
use std::sync::Arc;

use axum::middleware::from_fn;
use axum::Router;
use control::ApiKeyStore;
use mcp::http_auth::auth_middleware;
use mcp::McpServer;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpServerConfig, StreamableHttpService},
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("mcp=warn,rmcp=warn")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let token = std::env::var("MCP_HTTP_TOKEN")
        .ok()
        .filter(|s| !s.is_empty());
    let allow_insecure = std::env::var("MCP_HTTP_ALLOW_INSECURE")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    if token.is_none() && !allow_insecure {
        eprintln!("mcp-http: set MCP_HTTP_TOKEN (or MCP_HTTP_ALLOW_INSECURE=1 for local tests)");
        std::process::exit(2);
    }

    let addr: SocketAddr = std::env::var("MCP_HTTP_ADDR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080)));

    let ct = CancellationToken::new();
    let expected = token.clone();
    let api_keys = Arc::new(ApiKeyStore::new());
    let keys_for_service = Arc::clone(&api_keys);
    let service = StreamableHttpService::new(
        move || Ok(McpServer::with_api_keys(Arc::clone(&keys_for_service))),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig {
            cancellation_token: ct.child_token(),
            ..Default::default()
        },
    );

    let app = Router::new()
        .nest_service("/mcp", service)
        .layer(from_fn(move |req, next| {
            let expected = expected.clone();
            let api_keys = Arc::clone(&api_keys);
            async move { auth_middleware(expected, api_keys, req, next).await }
        }));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("mcp streamable-http listening on http://{addr}/mcp");

    let shutdown_ct = ct.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("mcp-http shutdown signal");
        shutdown_ct.cancel();
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
