//! Load eval.run golden fixtures (`sak532-a`).

use control::Offer;
use offer_eval::EvalRunOffer;
use serde_json::Value;
use types::{load_offer_fixture, BindingId, InvokeId, InvokeReq, InvokeResp};

fn load(name: &str) -> Value {
    load_offer_fixture(env!("CARGO_MANIFEST_DIR"), name).expect("fixture")
}

#[tokio::test]
async fn fixture_pass_fail_deny() {
    for name in [
        "eval.run.pass.json",
        "eval.run.fail.json",
        "eval.run.deny-assert.json",
    ] {
        let fix = load(name);
        assert_eq!(fix["schema"], "sak.fixture.offer/v0");
        let offer = EvalRunOffer::new().expect("offer");
        offer
            .bind(BindingId::new(), fix["bind_policy"].clone())
            .await
            .expect("bind");
        let resp = offer
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: fix["request"]["args"].clone(),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match fix["expect"]["status"].as_str() {
            Some("ok") => match resp {
                InvokeResp::Ok { result, .. } => {
                    assert_eq!(result["passed"], fix["expect"]["result"]["passed"]);
                }
                other @ InvokeResp::Error { .. } => {
                    panic!("{name}: expected ok, got {other:?}")
                }
            },
            Some("error") => match resp {
                InvokeResp::Error { code, .. } => {
                    assert_eq!(code.as_str(), fix["expect"]["code"].as_str().unwrap());
                }
                other @ InvokeResp::Ok { .. } => {
                    panic!("{name}: expected error, got {other:?}")
                }
            },
            other => panic!("bad expect.status {other:?}"),
        }
    }
}
