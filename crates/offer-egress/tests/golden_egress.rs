//! Load Nimbusware golden fixtures for egress (sak205).

#[cfg(test)]
mod tests {
    use control::Offer;
    use offer_egress::{enforce_response_bytes, EgressCheckOffer, ResponseByteCap};
    use serde_json::Value;
    use types::{load_offer_fixture, BindingId, ErrorCode, InvokeId, InvokeReq, InvokeResp};

    fn load(name: &str) -> Value {
        load_offer_fixture(env!("CARGO_MANIFEST_DIR"), name).expect("fixture")
    }

    #[tokio::test]
    async fn fixture_allow_and_deny() {
        for name in [
            "network.egress.check.allow.json",
            "network.egress.check.deny.json",
        ] {
            let fix = load(name);
            assert_eq!(fix["schema"], "sak.fixture.offer/v0");
            let offer = EgressCheckOffer::new().expect("offer");
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
                        assert_eq!(result["allowed"], fix["expect"]["result"]["allowed"]);
                        assert_eq!(result["host"], fix["expect"]["result"]["host"]);
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

    #[test]
    fn fixture_byte_cap_body() {
        let fix = load("network.egress.check.byte-cap.json");
        let cap = ResponseByteCap::from_policy(&fix["bind_policy"]);
        let n = usize::try_from(fix["synthetic_body_bytes"].as_u64().unwrap()).expect("usize");
        let body = vec![b'x'; n];
        assert_eq!(
            enforce_response_bytes(&cap, &body),
            Err(ErrorCode::BudgetExhausted)
        );
    }
}
