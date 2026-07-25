//! SwissArmyNoife HTTP admin binary (`sak363`).

use std::net::SocketAddr;

use http_admin::app;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let addr: SocketAddr = std::env::var("HTTP_ADDR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8787)));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind http");
    tracing::info!("swissarmynoife http listening on {addr}");
    axum::serve(listener, app()).await.expect("serve");
}
