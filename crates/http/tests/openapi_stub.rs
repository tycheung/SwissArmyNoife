//! `OpenAPI` stub presence (`sak323-a`).

#[test]
fn sak_admin_openapi_stub_exists() {
    // In-repo: SwissArmyNoife/docs (CI checkout has no Agentic parent docs/).
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/openapi/sak-admin.v0.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing openapi stub at {}: {e}", path.display()));
    assert!(text.contains("openapi:"));
    assert!(text.contains("/health"));
    assert!(text.contains("/v1/sak/health"));
    assert!(text.contains("/v1/sak/capacity"));
    assert!(text.contains("/v1/sak/compute/work"));
    assert!(text.contains("/v1/sak/compute/nodes"));
    assert!(text.contains("enum: [enqueue, claim, complete, get, list, requeue]"));
    assert!(text.contains("enum: [register, heartbeat, list]"));
    assert!(text.contains("session_id:"));
    assert!(text.contains("run_id:"));
    assert!(text.contains("stage_name:"));
    assert!(text.contains("stale_secs:"));
    assert!(text.contains("COMPUTE_QUEUE=sqlite"));
    assert!(text.contains("sak429") || text.contains("sak428"));
    assert!(text.contains("ComputeWorkListResponse"));
    assert!(text.contains("ComputeNodeListResponse"));
    assert!(text.contains("/v1/sak/bindings"));
    assert!(text.contains("/v1/sak/connections"));
    assert!(text.contains("/v1/sak/connections/{id}"));
    assert!(text.contains("/v1/sak/audit"));
    assert!(text.contains("/v1/sak/metrics"));
    assert!(text.contains("/v1/sak/metrics/prometheus"));
    assert!(text.contains("/metrics"));
    assert!(text.contains("/v1/chat/completions"));
    assert!(text.contains("bearerAuth"));
    assert!(text.contains("MCP_HTTP_TOKEN"));
}
