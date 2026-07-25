//! Load Nimbusware golden fixtures for memory index/search (`sak228`).

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use control::Offer;
    use offer_memory::{BackendKind, MemoryIndexOffer, MemoryPlane, MemorySearchOffer};
    use serde_json::{json, Value};
    use types::{load_nimbus_fixture, BindingId, InvokeId, InvokeReq, InvokeResp};

    fn load(name: &str) -> Value {
        load_nimbus_fixture(env!("CARGO_MANIFEST_DIR"), name).expect("fixture")
    }

    fn docs_from(fix: &Value) -> Value {
        json!({
            "documents": fix["documents"],
            "scope_key": fix.get("scope_key").cloned().unwrap_or(json!("default"))
        })
    }

    async fn index_with_backend(fix: &Value) -> (Arc<MemoryPlane>, MemorySearchOffer) {
        let plane = Arc::new(MemoryPlane::new());
        let backend = fix["backend"].as_str().unwrap_or("exact");
        let kind = BackendKind::parse(backend).unwrap_or(BackendKind::Exact);
        plane.set_backend(kind);
        let index = MemoryIndexOffer::new(Arc::clone(&plane)).expect("index");
        let search = MemorySearchOffer::new(Arc::clone(&plane)).expect("search");
        let bind_policy = json!({ "memory": { "backend": backend } });
        index
            .bind(BindingId::new(), bind_policy)
            .await
            .expect("bind index");
        let resp = index
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: docs_from(fix),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { .. } => {}
            other @ InvokeResp::Error { .. } => panic!("index failed: {other:?}"),
        }
        (plane, search)
    }

    #[tokio::test]
    async fn fixture_search_rank_exact() {
        let fix = load("memory.search.rank.json");
        assert_eq!(fix["schema"], "sak.fixture.nimbusware/v0");
        let (_plane, search) = index_with_backend(&fix).await;
        let resp = search
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: fix["request"]["args"].clone(),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match resp {
            InvokeResp::Ok { result, .. } => {
                let hits = result["hits"].as_array().expect("hits");
                assert!(!hits.is_empty());
                let id = hits[0]["id"].as_str().unwrap();
                let prefix = fix["expect"]["top_id_prefix"].as_str().unwrap();
                assert!(id.starts_with(prefix), "id={id} prefix={prefix}");
                let excerpt = hits[0]["excerpt"].as_str().unwrap();
                assert!(excerpt.contains(fix["expect"]["excerpt_contains"].as_str().unwrap()));
            }
            other @ InvokeResp::Error { .. } => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn fixture_index_fingerprint_skip() {
        let fix = load("memory.index.skip.json");
        let plane = Arc::new(MemoryPlane::new());
        let index = MemoryIndexOffer::new(Arc::clone(&plane)).expect("index");
        let args = docs_from(&fix);
        let first = index
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args: args.clone(),
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        let second = index
            .invoke(InvokeReq {
                binding_id: BindingId::new(),
                args,
                invoke_id: Some(InvokeId::new()),
                offer: None,
            })
            .await;
        match (first, second) {
            (InvokeResp::Ok { result: a, .. }, InvokeResp::Ok { result: b, .. }) => {
                assert_eq!(a["rebuilt"], fix["expect"]["first_rebuilt"]);
                assert_eq!(b["rebuilt"], fix["expect"]["second_rebuilt"]);
                assert_eq!(a["fingerprint"], b["fingerprint"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn fixture_search_hnsw_and_faiss_stub() {
        for name in ["memory.search.hnsw.json", "memory.search.faiss-stub.json"] {
            let fix = load(name);
            let (plane, search) = index_with_backend(&fix).await;
            if let Some(want) = fix["expect"]["backend"].as_str() {
                let (_, backend, _) = plane.meta().expect("meta");
                assert_eq!(backend, want, "{name}");
            }
            let resp = search
                .invoke(InvokeReq {
                    binding_id: BindingId::new(),
                    args: fix["request"]["args"].clone(),
                    invoke_id: Some(InvokeId::new()),
                    offer: None,
                })
                .await;
            match resp {
                InvokeResp::Ok { result, .. } => {
                    let hits = result["hits"].as_array().expect("hits");
                    let id = hits[0]["id"].as_str().unwrap();
                    let prefix = fix["expect"]["top_id_prefix"].as_str().unwrap();
                    assert!(id.starts_with(prefix), "{name}: id={id}");
                    let excerpt = hits[0]["excerpt"].as_str().unwrap();
                    assert!(
                        excerpt.contains(fix["expect"]["excerpt_contains"].as_str().unwrap()),
                        "{name}"
                    );
                }
                other @ InvokeResp::Error { .. } => panic!("{name}: {other:?}"),
            }
        }
    }
}
