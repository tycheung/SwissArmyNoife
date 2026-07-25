//! Streamable HTTP MCP smoke: bearer auth + ping (`sak110-c`).

use std::net::SocketAddr;
use std::time::Duration;

use rmcp::{
    model::CallToolRequestParam,
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig, StreamableHttpClientTransport,
    },
    ServiceExt,
};
use tokio::process::{Child, Command};
use tokio::time::sleep;

#[tokio::test]
async fn http_ping_requires_bearer_and_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_mcp-http");
    let tmp = tempfile::tempdir()?;
    let addr = free_addr().await?;
    let token = "sak110-test-token";

    let mut child = spawn_http(bin, &tmp, addr, token)?;
    wait_ready(addr).await?;

    let uri = format!("http://{addr}/mcp");
    let denied = reqwest::Client::new()
        .post(&uri)
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .await?;
    assert_eq!(denied.status(), reqwest::StatusCode::UNAUTHORIZED);

    let transport = StreamableHttpClientTransport::with_client(
        reqwest::Client::new(),
        StreamableHttpClientTransportConfig::with_uri(uri).auth_header(token),
    );
    let client = ().serve(transport).await?;
    let pong = client
        .call_tool(CallToolRequestParam {
            name: "ping".into(),
            arguments: None,
        })
        .await?;
    let text: String = pong
        .content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
        .collect();
    assert_eq!(text, "ok");

    let _ = client.cancel().await;
    let _ = child.kill().await;
    Ok(())
}

async fn free_addr() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr)
}

fn spawn_http(
    bin: &str,
    tmp: &tempfile::TempDir,
    addr: SocketAddr,
    token: &str,
) -> Result<Child, Box<dyn std::error::Error>> {
    Ok(Command::new(bin)
        .env("CONFIG_DIR", tmp.path())
        .env("LLM_BACKEND", "echo")
        .env("SANDBOX_BACKEND", "none")
        .env("CAPACITY_PROBE", "fake")
        .env("MCP_HTTP_ADDR", addr.to_string())
        .env("MCP_HTTP_TOKEN", token)
        .kill_on_drop(true)
        .spawn()?)
}

async fn wait_ready(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("http://{addr}/mcp");
    for _ in 0..50 {
        if reqwest::Client::new()
            .post(&url)
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .is_ok()
        {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err("mcp-http did not become ready".into())
}
