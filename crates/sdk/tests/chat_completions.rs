//! `SakClient::chat_completions` wiremock coverage (`sak545-b`).

use serde_json::json;
use sdk::SakClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn chat_completions_posts_facade_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "echo:hi" },
                "finish_reason": "stop"
            }]
        })))
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    let v = client
        .chat_completions(json!({
            "binding_id": "00000000-0000-0000-0000-000000000001",
            "model": "echo",
            "messages": [{ "role": "user", "content": "hi" }]
        }))
        .await
        .expect("chat");
    assert_eq!(v["object"], "chat.completion");
    assert_eq!(v["choices"][0]["message"]["content"], "echo:hi");
}
