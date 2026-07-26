//! Load Nimbusware sandbox golden fixtures (`sak159-a` / `sak159-b` / `sak159-c`).

use control::Offer;
use offer_sandbox::{ExecRequest, NoneBackend, SandboxBackend, SandboxExecOffer, StubBackend};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use types::{load_offer_fixture, BindingId, ErrorCode, InvokeReq, InvokeResp};

#[test]
fn path_escape_fixture_expectation_string() {
    let fix = load_offer_fixture(env!("CARGO_MANIFEST_DIR"), "sandbox/path-escape.json")
        .expect("fixture");
    assert_eq!(fix["schema"], "sak.fixture.offer/v0");
    let needle = fix["expect"]["message_contains"]
        .as_str()
        .expect("message_contains");
    assert_eq!(needle, "sandbox.violation:path_escape");

    let tmp = TempDir::new().expect("tempdir");
    let backend = NoneBackend::with_root(tmp.path()).expect("backend");
    let err = backend
        .exec(&ExecRequest {
            argv: vec!["echo".into(), "x".into()],
            cwd: PathBuf::from(".."),
        })
        .expect_err("escape");
    let message = err.to_string();
    assert!(message.contains(needle), "expected {needle:?} in {message}");
}

#[tokio::test]
async fn argv_empty_fixture_schema_invalid() {
    let fix =
        load_offer_fixture(env!("CARGO_MANIFEST_DIR"), "sandbox/argv-empty.json").expect("fixture");
    assert_eq!(fix["schema"], "sak.fixture.offer/v0");
    let needle = fix["expect"]["message_contains"]
        .as_str()
        .expect("message_contains");
    assert_eq!(fix["expect"]["code"], "schema.invalid");

    let tmp = TempDir::new().expect("tempdir");
    let backend = StubBackend::with_root(tmp.path()).expect("backend");
    let offer = SandboxExecOffer::with_policy(backend, &json!({})).expect("offer");
    let args = fix["request"]["args"].clone();
    match offer
        .invoke(InvokeReq {
            binding_id: BindingId::new(),
            args,
            invoke_id: None,
            offer: None,
        })
        .await
    {
        InvokeResp::Error {
            code: ErrorCode::SchemaInvalid,
            message,
            ..
        } => {
            assert!(message.contains(needle), "expected {needle:?} in {message}");
        }
        other => panic!("expected schema.invalid, got {other:?}"),
    }
}

fn outside_jail_absolute_cwd() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\Windows\System32")
    } else {
        PathBuf::from("/etc/passwd")
    }
}

#[test]
fn absolute_cwd_escape_fixture_expectation_string() {
    let fix = load_offer_fixture(
        env!("CARGO_MANIFEST_DIR"),
        "sandbox/absolute-cwd-escape.json",
    )
    .expect("fixture");
    assert_eq!(fix["schema"], "sak.fixture.offer/v0");
    let needle = fix["expect"]["message_contains"]
        .as_str()
        .expect("message_contains");
    assert_eq!(needle, "sandbox.violation:path_escape");
    assert_eq!(fix["expect"]["code"], "sandbox.violation");

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonicalize");
    let outside = outside_jail_absolute_cwd();
    if outside.starts_with(&root) {
        return;
    }
    let backend = NoneBackend::with_root(&root).expect("backend");
    let err = backend
        .exec(&ExecRequest {
            argv: vec!["echo".into(), "x".into()],
            cwd: outside,
        })
        .expect_err("absolute outside jail");
    let message = err.to_string();
    assert!(message.contains(needle), "expected {needle:?} in {message}");
}
