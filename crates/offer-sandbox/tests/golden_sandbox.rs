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

#[test]
fn argv_outside_fixture() {
    let fix = load_offer_fixture(env!("CARGO_MANIFEST_DIR"), "sandbox/argv-outside.json")
        .expect("fixture");
    let needle = fix["expect"]["message_contains"]
        .as_str()
        .expect("message_contains");
    let tmp = TempDir::new().expect("tempdir");
    let backend = StubBackend::with_root(tmp.path()).expect("backend");
    let err = backend
        .exec(&ExecRequest {
            argv: vec!["cat".into(), "../secret".into()],
            cwd: PathBuf::from("."),
        })
        .expect_err("outside");
    let message = err.to_string();
    assert!(message.contains(needle), "expected {needle:?} in {message}");
}

#[test]
fn symlink_escape_fixture() {
    let fix = load_offer_fixture(env!("CARGO_MANIFEST_DIR"), "sandbox/symlink-escape.json")
        .expect("fixture");
    let needle = fix["expect"]["message_contains"]
        .as_str()
        .expect("message_contains");
    let tmp = TempDir::new().expect("tempdir");
    let outside = std::env::temp_dir().join("sak551-golden-outside");
    let _ = std::fs::create_dir_all(&outside);
    let link = tmp.path().join("escape-link");
    let linked = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, &link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&outside, &link).is_ok()
        }
    };
    if !linked {
        eprintln!("sak551-c: skip symlink golden (create failed)");
        return;
    }
    let backend = StubBackend::with_root(tmp.path()).expect("backend");
    let err = backend
        .exec(&ExecRequest {
            argv: vec!["echo".into(), "x".into()],
            cwd: PathBuf::from("escape-link"),
        })
        .expect_err("symlink escape");
    let message = err.to_string();
    assert!(message.contains(needle), "expected {needle:?} in {message}");
}

#[tokio::test]
async fn shell_wrapper_deny_fixture() {
    let fix = load_offer_fixture(
        env!("CARGO_MANIFEST_DIR"),
        "sandbox/shell-wrapper-deny.json",
    )
    .expect("fixture");
    let needle = fix["expect"]["message_contains"]
        .as_str()
        .expect("message_contains");
    assert_eq!(fix["expect"]["code"], "policy.denied");

    let tmp = TempDir::new().expect("tempdir");
    let backend = StubBackend::with_root(tmp.path()).expect("backend");
    let offer = SandboxExecOffer::with_policy(backend, &json!({})).expect("offer");
    match offer
        .invoke(InvokeReq {
            binding_id: BindingId::new(),
            args: fix["request"]["args"].clone(),
            invoke_id: None,
            offer: None,
        })
        .await
    {
        InvokeResp::Error {
            code: ErrorCode::PolicyDenied,
            message,
            ..
        } => {
            assert!(message.contains(needle), "expected {needle:?} in {message}");
        }
        other => panic!("expected policy.denied, got {other:?}"),
    }
}
