//! sak430-h: SakClient.requeue_work posts expected body.

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use sdk::SakClient;

#[tokio::test]
async fn requeue_work_posts_action_requeue() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/sak/compute/work"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "action": "requeue",
            "work": { "id": "w1", "status": "queued" },
            "via": "app_state_compute_plane"
        })))
        .mount(&server)
        .await;

    let client = SakClient::new(server.uri());
    let out = client.requeue_work("w1").await.expect("requeue");
    assert_eq!(out["action"], "requeue");
    assert_eq!(out["work"]["id"], "w1");
}
