//! Live HTTP guarded GET against wiremock (sak202-e).

use offer_egress::{guarded_get, EgressPolicy, ReqwestGet};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn wiremock_guarded_get_ok() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hi"))
        .respond_with(ResponseTemplate::new(200).set_body_string("wire-ok"))
        .mount(&server)
        .await;

    let host = offer_egress::host_from_url(&server.uri()).expect("host");
    let policy = EgressPolicy::from_policy(&json!({
        "egress": {
            "allow_hosts": [host],
            "allow_principals": ["local"],
            "max_response_bytes": 1024
        }
    }));
    let url = format!("{}/hi", server.uri());
    let body = guarded_get(&policy, "local", &url, &ReqwestGet::new())
        .await
        .expect("get");
    assert_eq!(body.status, 200);
    assert_eq!(body.bytes, b"wire-ok");
}
