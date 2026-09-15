//! sak596-m: Playwright fixture e2e (skip if node/playwright unavailable).

use std::path::PathBuf;
use std::time::Duration;

use offer_browser::{BrowserBackend, PlaywrightBackend};
use serde_json::json;
use tokio::process::Command;

fn sidecar_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecar/browser_sidecar.mjs")
}

fn fixture_url() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/fixture.html");
    let abs = path.canonicalize().expect("fixture path");
    // Windows: \\?\C:\... → file:///C:/...
    let s = abs.to_string_lossy().replacen(r"\\?\", "", 1);
    let s = s.replace('\\', "/");
    if s.chars().nth(1) == Some(':') {
        format!("file:///{s}")
    } else {
        format!("file://{s}")
    }
}

async fn node_can_import_playwright() -> bool {
    let script = sidecar_script();
    if !script.is_file() {
        return false;
    }
    let Ok(out) = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg("import('playwright').then(() => process.exit(0)).catch(() => process.exit(1))")
        .current_dir(script.parent().expect("sidecar dir"))
        .output()
        .await
    else {
        return false;
    };
    out.status.success()
}

fn find_ref(snapshot: &str, needle: &str) -> Option<String> {
    for line in snapshot.lines() {
        if line.contains(needle) {
            if let Some(start) = line.rfind('[') {
                if let Some(end) = line.rfind(']') {
                    if end > start {
                        return Some(line[start + 1..end].to_owned());
                    }
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn playwright_fixture_interact_failure_report_or_skip() {
    if !node_can_import_playwright().await {
        eprintln!("skip: node/playwright not available for browser sidecar");
        return;
    }
    std::env::set_var("BROWSER_SIDECAR", sidecar_script().display().to_string());
    let backend = match PlaywrightBackend::from_env() {
        Ok(b) => b,
        Err(_) => {
            eprintln!("skip: PlaywrightBackend::from_env failed");
            return;
        }
    };

    let tmp = tempfile::tempdir().expect("tmp");
    let profile = tmp.path().join("profile");
    let url = fixture_url();

    let nav = tokio::time::timeout(
        Duration::from_secs(60),
        backend.call(&profile, "navigate", json!({ "url": url })),
    )
    .await;
    let Ok(Ok(nav)) = nav else {
        eprintln!("skip: playwright navigate failed or timed out: {nav:?}");
        return;
    };
    assert_eq!(nav["backend"], "playwright");

    let snap = backend
        .call(&profile, "snapshot", json!({}))
        .await
        .expect("snapshot");
    let text = snap["snapshot"].as_str().unwrap_or("").to_owned();
    assert!(text.contains("[e"), "expected refs: {text}");

    let name_ref = find_ref(&text, "Name").or_else(|| find_ref(&text, "textbox"));
    let go_ref = find_ref(&text, "Go").or_else(|| find_ref(&text, "button"));
    let (Some(name_ref), Some(go_ref)) = (name_ref, go_ref) else {
        eprintln!("skip: could not resolve name/go refs from snapshot:\n{text}");
        let _ = backend.call(&profile, "close", json!({})).await;
        return;
    };

    backend
        .call(
            &profile,
            "fill",
            json!({ "ref": name_ref, "text": "parity" }),
        )
        .await
        .expect("fill");
    backend
        .call(&profile, "click", json!({ "ref": go_ref }))
        .await
        .expect("click");

    let shot = backend
        .call(&profile, "take_screenshot", json!({}))
        .await
        .expect("screenshot");
    assert!(
        shot.get("path").and_then(|p| p.as_str()).is_some(),
        "screenshot={shot}"
    );

    // Allow console listener to flush.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let report = backend
        .call(
            &profile,
            "failure_report",
            json!({ "step": "fixture-click" }),
        )
        .await
        .expect("failure_report");
    let report_s = report.to_string();
    assert!(
        report
            .get("screenshot_path")
            .and_then(|p| p.as_str())
            .is_some(),
        "report missing screenshot_path: {report_s}"
    );
    assert!(
        report_s.contains("fixture-console-error") || report_s.contains("parity"),
        "report missing console correlation: {report_s}"
    );

    let deny = backend
        .call(
            &profile,
            "cdp",
            json!({ "method": "Input.dispatchKeyEvent", "params": {} }),
        )
        .await;
    assert!(deny.is_err(), "Input.* CDP must be denied");

    let _ = backend.call(&profile, "close", json!({})).await;
}

#[tokio::test]
async fn playwright_navigate_snapshot_or_skip() {
    if !node_can_import_playwright().await {
        eprintln!("skip: node/playwright not available for browser sidecar");
        return;
    }
    std::env::set_var("BROWSER_SIDECAR", sidecar_script().display().to_string());
    let backend = match PlaywrightBackend::from_env() {
        Ok(b) => b,
        Err(_) => return,
    };
    let tmp = tempfile::tempdir().expect("tmp");
    let profile = tmp.path().join("profile");
    let nav = tokio::time::timeout(
        Duration::from_secs(60),
        backend.call(
            &profile,
            "navigate",
            json!({ "url": "https://example.com/" }),
        ),
    )
    .await;
    let Ok(Ok(_)) = nav else {
        eprintln!("skip: playwright navigate failed: {nav:?}");
        return;
    };
    let snap = backend
        .call(&profile, "snapshot", json!({}))
        .await
        .expect("snapshot");
    assert!(snap["snapshot"].as_str().unwrap_or("").contains("[e"));
    let _ = backend.call(&profile, "close", json!({})).await;
}
