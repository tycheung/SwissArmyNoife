//! `mcp` — Model Context Protocol edge (stdio).
//!
//! Logs go to stderr only; stdout is reserved for MCP JSON-RPC.

use mcp::McpServer;
use rmcp::{transport::stdio, ServiceExt};
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

    tracing::info!("mcp starting (stdio)");
    let service = McpServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
