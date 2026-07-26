//! Optional MCP client wrapping [`rmcp`] Streamable HTTP (`sak348-b`).

use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    service::{RunningService, ServiceExt},
    transport::streamable_http_client::StreamableHttpClientTransportConfig,
    transport::StreamableHttpClientTransport,
    RoleClient,
};
use serde_json::{json, Value};

use crate::SdkError;

/// Thin `SwissArmyNoife` MCP client over Streamable HTTP via `rmcp`.
pub struct SakMcpClient {
    service: RunningService<RoleClient, ()>,
}

impl SakMcpClient {
    /// Connect to `mcp_url` (e.g. `http://127.0.0.1:8080/mcp`) and complete initialize.
    ///
    /// # Errors
    /// Transport / initialize failures.
    pub async fn connect(mcp_url: impl Into<String>) -> Result<Self, SdkError> {
        Self::connect_inner(mcp_url.into(), None).await
    }

    pub(crate) async fn connect_inner(
        mcp_url: String,
        token: Option<String>,
    ) -> Result<Self, SdkError> {
        let mut config = StreamableHttpClientTransportConfig::with_uri(mcp_url);
        if let Some(token) = token {
            config = config.auth_header(token);
        }
        let transport = StreamableHttpClientTransport::from_config(config);
        let service = ().serve(transport).await.map_err(|e| SdkError::Http(e.to_string()))?;
        Ok(Self { service })
    }

    /// MCP `tools/call` `ping` — returns first text content (usually `"pong"`).
    ///
    /// # Errors
    /// Tool call / empty content.
    pub async fn ping(&self) -> Result<String, SdkError> {
        let result = self.call_tool("ping", None).await?;
        extract_ping_text(&result)
    }

    /// MCP `tools/list` (all pages).
    ///
    /// # Errors
    /// List failure.
    pub async fn tools_list(&self) -> Result<Value, SdkError> {
        let tools = self
            .service
            .list_all_tools()
            .await
            .map_err(|e| SdkError::Http(e.to_string()))?;
        serde_json::to_value(json!({ "tools": tools })).map_err(|e| SdkError::Schema(e.to_string()))
    }

    /// MCP `tools/call` `catalog_list`.
    ///
    /// # Errors
    /// Tool call failure.
    pub async fn catalog_list(&self) -> Result<Value, SdkError> {
        let result = self.call_tool("catalog_list", None).await?;
        serde_json::to_value(result).map_err(|e| SdkError::Schema(e.to_string()))
    }

    async fn call_tool(
        &self,
        name: &'static str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<CallToolResult, SdkError> {
        self.service
            .call_tool(CallToolRequestParam {
                name: name.into(),
                arguments,
            })
            .await
            .map_err(|e| SdkError::Http(e.to_string()))
    }
}

fn extract_ping_text(result: &CallToolResult) -> Result<String, SdkError> {
    for item in &result.content {
        if let Some(text) = item.as_text() {
            return Ok(text.text.clone());
        }
    }
    Err(SdkError::Schema("ping: no text content".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_mcp() -> MockServer {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(|req: &wiremock::Request| {
                let body: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
                let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");
                let id = body.get("id").cloned().unwrap_or(json!(1));
                match method {
                    "initialize" => ResponseTemplate::new(200)
                        .insert_header("content-type", "application/json")
                        .insert_header("mcp-session-id", "sess-sak348-b")
                        .set_body_json(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "protocolVersion": "2025-03-26",
                                "capabilities": { "tools": {} },
                                "serverInfo": { "name": "mock", "version": "0.0.1" }
                            }
                        })),
                    "notifications/initialized" => ResponseTemplate::new(202),
                    "tools/list" => ResponseTemplate::new(200)
                        .insert_header("content-type", "application/json")
                        .set_body_json(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "tools": [
                                    {
                                        "name": "ping",
                                        "inputSchema": { "type": "object", "properties": {} }
                                    },
                                    {
                                        "name": "catalog_list",
                                        "inputSchema": { "type": "object", "properties": {} }
                                    }
                                ]
                            }
                        })),
                    "tools/call" => {
                        let name = body
                            .pointer("/params/name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("");
                        let text = match name {
                            "ping" => "pong",
                            "catalog_list" => "{\"offers\":[]}",
                            other => other,
                        };
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "application/json")
                            .set_body_json(json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{ "type": "text", "text": text }]
                                }
                            }))
                    }
                    other => {
                        ResponseTemplate::new(500).set_body_string(format!("bad method {other}"))
                    }
                }
            })
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(405))
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(405))
            .mount(&server)
            .await;

        server
    }

    #[tokio::test]
    async fn connect_ping_tools_list_catalog_list() {
        let server = mock_mcp().await;
        let url = format!("{}/mcp", server.uri());
        let client = SakMcpClient::connect(url).await.expect("connect");
        assert_eq!(client.ping().await.expect("ping"), "pong");
        let tools = client.tools_list().await.expect("tools_list");
        let names: Vec<&str> = tools["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"ping"));
        assert!(names.contains(&"catalog_list"));
        let catalog = client.catalog_list().await.expect("catalog_list");
        let text = catalog["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("offers"));
    }
}
