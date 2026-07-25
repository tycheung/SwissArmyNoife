//! Shared loader for Nimbusware golden fixtures under `fixtures/nimbusware/`.

use serde_json::Value;

/// Load `{crate_manifest_dir}/../../fixtures/nimbusware/{name}` as JSON.
///
/// # Errors
/// I/O or JSON parse failures as a display string.
pub fn load_nimbus_fixture(crate_manifest_dir: &str, name: &str) -> Result<Value, String> {
    let path = format!("{crate_manifest_dir}/../../fixtures/nimbusware/{name}");
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("json {path}: {e}"))
}
