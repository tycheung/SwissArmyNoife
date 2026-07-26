//! OpenAI-shaped chat facade example (`sak545-c`).
//!
//! Needs a live `http-admin` and a pre-bound `llm.chat` UUID in `SAK_LLM_BINDING`.
//!
//! ```bash
//! cargo run -p http-admin
//! SAK_LLM_BINDING=<uuid> cargo run -p sdk --example chat_completions
//! ```

use sdk::SakClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("SAK_HTTP").unwrap_or_else(|_| "http://127.0.0.1:8787".into());
    let binding = std::env::var("SAK_LLM_BINDING").map_err(|_| {
        "set SAK_LLM_BINDING to an llm.chat binding UUID (MCP bind or test helper)"
    })?;

    let client = SakClient::new(base);
    let chat = client
        .chat_completions(json!({
            "binding_id": binding,
            "model": "echo",
            "messages": [{ "role": "user", "content": "ping" }]
        }))
        .await?;
    println!("chat: {chat}");
    Ok(())
}
