//! Optional MCP client wrapping [`rmcp`] Streamable HTTP (`sak348-b`…`d`).

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
///
/// After [`Self::connect`], `rmcp` owns the Streamable HTTP session (`mcp-session-id`),
/// matching TS/Python initialize + session header behavior (`sak348-c`).
pub struct SakMcpClient {
    service: RunningService<RoleClient, ()>,
}

impl SakMcpClient {
    /// Connect to `mcp_url` (e.g. `http://127.0.0.1:8080/mcp`) and complete initialize.
    ///
    /// Uses `MCP_HTTP_TOKEN` when set (non-empty). Prefer [`Self::connect_with_token`] to
    /// pass a bearer token explicitly.
    ///
    /// # Errors
    /// Transport / initialize failures.
    pub async fn connect(mcp_url: impl Into<String>) -> Result<Self, SdkError> {
        let token = std::env::var("MCP_HTTP_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self::connect_inner(mcp_url.into(), token).await
    }

    /// Connect with an explicit bearer token (no `Bearer ` prefix).
    ///
    /// # Errors
    /// Transport / initialize failures.
    pub async fn connect_with_token(
        mcp_url: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, SdkError> {
        Self::connect_inner(mcp_url.into(), Some(token.into())).await
    }

    async fn connect_inner(mcp_url: String, token: Option<String>) -> Result<Self, SdkError> {
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
        self.tools_call_value("catalog_list", None).await
    }

    /// MCP `bind` — returns tool result JSON (`sak348-d` / sak329-c).
    ///
    /// # Errors
    /// Tool call failure.
    pub async fn bind(&self, offer_id: &str) -> Result<Value, SdkError> {
        self.tools_call_value("bind", Some(json!({ "offer_id": offer_id })))
            .await
    }

    /// MCP `invoke` — invoke a bound offer (`sak348-d` / sak329-c).
    ///
    /// # Errors
    /// Tool call failure.
    pub async fn invoke(
        &self,
        binding_id: &str,
        args: Option<Value>,
        offer: Option<&str>,
    ) -> Result<Value, SdkError> {
        let mut params = json!({
            "binding_id": binding_id,
            "args": args.unwrap_or_else(|| json!({})),
        });
        if let Some(offer) = offer {
            params["offer"] = json!(offer);
        }
        self.tools_call_value("invoke", Some(params)).await
    }

    /// MCP `provision` — provision an offer resource (`sak348-d` / sak329-c).
    ///
    /// # Errors
    /// Tool call failure.
    pub async fn provision(
        &self,
        offer_id: &str,
        idempotency_key: Option<&str>,
    ) -> Result<Value, SdkError> {
        let mut params = json!({ "offer_id": offer_id });
        if let Some(key) = idempotency_key {
            params["idempotency_key"] = json!(key);
        }
        self.tools_call_value("provision", Some(params)).await
    }

    async fn tools_call_value(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, SdkError> {
        let map = match arguments {
            None | Some(Value::Null) => None,
            Some(Value::Object(m)) => Some(m),
            Some(other) => {
                return Err(SdkError::Schema(format!(
                    "tools/call {name}: arguments must be object, got {other}"
                )));
            }
        };
        let result = self.call_tool(name, map).await?;
        serde_json::to_value(result).map_err(|e| SdkError::Schema(e.to_string()))
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<CallToolResult, SdkError> {
        self.service
            .call_tool(CallToolRequestParam {
                name: name.to_string().into(),
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
                        .insert_header("mcp-session-id", "sess-sak348")
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
                            "ping" => "pong".to_string(),
                            "catalog_list" => "{\"offers\":[]}".to_string(),
                            "bind" => "{\"binding_id\":\"b1\",\"offer_id\":\"o1\"}".to_string(),
                            "invoke" => "{\"status\":\"ok\"}".to_string(),
                            "provision" => "{\"status\":\"provisioned\"}".to_string(),
                            other => other.to_string(),
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

    fn rpc_method(req: &wiremock::Request) -> String {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
        body.get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string()
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

    #[tokio::test]
    async fn session_header_on_tools_call_after_initialize() {
        let server = mock_mcp().await;
        let url = format!("{}/mcp", server.uri());
        let client = SakMcpClient::connect(url).await.expect("connect");
        client.ping().await.expect("ping");
        let requests = server.received_requests().await.expect("requests");
        let ping = requests
            .iter()
            .find(|r| r.method == "POST" && rpc_method(r) == "tools/call")
            .expect("tools/call");
        let sid = ping
            .headers
            .get("mcp-session-id")
            .expect("mcp-session-id")
            .to_str()
            .expect("utf8");
        assert_eq!(sid, "sess-sak348");
    }

    #[tokio::test]
    async fn connect_with_token_sends_bearer_auth() {
        let server = mock_mcp().await;
        let url = format!("{}/mcp", server.uri());
        let client = SakMcpClient::connect_with_token(url, "tok-348c")
            .await
            .expect("connect");
        client.ping().await.expect("ping");
        let requests = server.received_requests().await.expect("requests");
        let init = requests
            .iter()
            .find(|r| r.method == "POST" && rpc_method(r) == "initialize")
            .expect("initialize");
        let auth = init
            .headers
            .get("authorization")
            .expect("authorization")
            .to_str()
            .expect("utf8");
        assert_eq!(auth, "Bearer tok-348c");
    }

    #[tokio::test]
    async fn bind_invoke_provision_post_tools_call() {
        let server = mock_mcp().await;
        let url = format!("{}/mcp", server.uri());
        let client = SakMcpClient::connect(url).await.expect("connect");

        let bound = client.bind("offer.llm").await.expect("bind");
        assert!(bound["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("binding_id"));

        let invoked = client
            .invoke("b1", Some(json!({ "prompt": "hi" })), Some("offer.llm"))
            .await
            .expect("invoke");
        assert!(invoked["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("ok"));

        let provisioned = client
            .provision("offer.llm", Some("idem-1"))
            .await
            .expect("provision");
        assert!(provisioned["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("provisioned"));

        let requests = server.received_requests().await.expect("requests");
        let calls: Vec<Value> = requests
            .iter()
            .filter(|r| r.method == "POST" && rpc_method(r) == "tools/call")
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .collect();
        let names: Vec<&str> = calls
            .iter()
            .filter_map(|b| b.pointer("/params/name").and_then(|n| n.as_str()))
            .collect();
        assert_eq!(names, vec!["bind", "invoke", "provision"]);
        assert_eq!(
            calls[1]
                .pointer("/params/arguments/offer")
                .and_then(|v| v.as_str()),
            Some("offer.llm")
        );
        assert_eq!(
            calls[2]
                .pointer("/params/arguments/idempotency_key")
                .and_then(|v| v.as_str()),
            Some("idem-1")
        );
    }
}
