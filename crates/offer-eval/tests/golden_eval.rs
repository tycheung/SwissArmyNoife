//! Eval golden fixtures (`sak532-a` offers + `sak535-b` `fixtures/eval`).

use control::Offer;
use offer_eval::EvalRunOffer;
use serde_json::Value;
use types::{load_offer_fixture, BindingId, InvokeId, InvokeReq, InvokeResp};

fn load_offers(name: &str) -> Value {
    load_offer_fixture(env!("CARGO_MANIFEST_DIR"), name).expect("fixture")
}

fn load_eval_dir(name: &str) -> Value {
    let path = format!(
        "{}/../../fixtures/eval/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("json {path}: {e}"))
}

async fn run_fixture(name: &str, fix: Value) {
    assert_eq!(fix["schema"], "sak.fixture.offer/v0", "{name}");
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
                if let Some(passed) = fix["expect"]["result"]["passed"].as_bool() {
                    assert_eq!(result["passed"], passed, "{name}");
                }
                if let Some(len) = fix["expect"]["results_len"].as_u64() {
                    let want = usize::try_from(len).expect("results_len");
                    assert_eq!(
                        result["results"].as_array().map(Vec::len),
                        Some(want),
                        "{name} results_len"
                    );
                }
            }
            other @ InvokeResp::Error { .. } => {
                panic!("{name}: expected ok, got {other:?}")
            }
        },
        Some("error") => match resp {
            InvokeResp::Error { code, .. } => {
                assert_eq!(
                    code.as_str(),
                    fix["expect"]["code"].as_str().unwrap(),
                    "{name}"
                );
            }
            other @ InvokeResp::Ok { .. } => {
                panic!("{name}: expected error, got {other:?}")
            }
        },
        other => panic!("{name}: bad expect.status {other:?}"),
    }
}

#[tokio::test]
async fn fixture_pass_fail_deny() {
    for name in [
        "eval.run.pass.json",
        "eval.run.fail.json",
        "eval.run.deny-assert.json",
    ] {
        run_fixture(name, load_offers(name)).await;
    }
}

#[tokio::test]
async fn fixtures_eval_pack() {
    let dir = format!("{}/../../fixtures/eval", env!("CARGO_MANIFEST_DIR"));
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {dir}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            std::path::Path::new(n)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    names.sort();
    assert!(
        names.len() >= 8,
        "expected eval fixture pack, got {names:?}"
    );
    for name in names {
        run_fixture(&name, load_eval_dir(&name)).await;
    }
}
