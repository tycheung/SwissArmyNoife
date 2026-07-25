//! Ensures Nimbusware golden fixtures stay parseable as wire types.

#[cfg(test)]
mod tests {
    use types::InvokeReq;

    #[test]
    fn llm_chat_fixture_request_deserializes() {
        let raw = include_str!("../../../fixtures/nimbusware/llm.chat.roundtrip.json");
        let v: serde_json::Value = serde_json::from_str(raw).expect("fixture json");
        assert_eq!(v["schema"], "sak.fixture.nimbusware/v0");
        assert_eq!(v["offer"], "llm.chat");
        let req: InvokeReq =
            serde_json::from_value(v["request"].clone()).expect("InvokeReq from fixture");
        assert_eq!(
            req.offer.as_ref().map(types::OfferId::as_str),
            Some("llm.chat")
        );
        assert!(req.args.get("messages").is_some());
    }
}
