//! Client-side package integrity / HMAC signature verify (`sak357`).

use std::fs;
use std::path::Path;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use types::ErrorCode;

type HmacSha256 = Hmac<Sha256>;

/// Well-known OSS smoke key (not for production signing).
pub const SMOKE_HMAC_KEY: &[u8] = b"swissarmynoife-module-smoke-key-v0";

/// Filename for optional package signature (hex HMAC-SHA256 of [`package_digest`]).
pub const SIGNATURE_FILE: &str = "signature.hmac";

/// Canonical digest of `manifest.toml` + payload bytes (order fixed).
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on I/O / missing files.
pub fn package_digest(package_dir: &Path, payload_rel: &str) -> Result<[u8; 32], ErrorCode> {
    let manifest =
        fs::read(package_dir.join("manifest.toml")).map_err(|_| ErrorCode::SchemaInvalid)?;
    let payload = fs::read(package_dir.join(payload_rel)).map_err(|_| ErrorCode::SchemaInvalid)?;
    let mut hasher = Sha256::new();
    hasher.update(b"manifest.toml\0");
    hasher.update(&manifest);
    hasher.update(b"\0payload\0");
    hasher.update(payload_rel.as_bytes());
    hasher.update(b"\0");
    hasher.update(&payload);
    Ok(hasher.finalize().into())
}

/// Hex-encode digest / signature bytes.
#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Decode hex (even length).
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on bad hex.
pub fn hex_decode(hex: &str) -> Result<Vec<u8>, ErrorCode> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return Err(ErrorCode::SchemaInvalid);
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ErrorCode::SchemaInvalid))
        .collect()
}

/// Compute HMAC-SHA256 signature hex for a package.
///
/// # Errors
/// Digest / HMAC construction errors as schema/incompatible.
pub fn sign_package(
    package_dir: &Path,
    payload_rel: &str,
    key: &[u8],
) -> Result<String, ErrorCode> {
    let digest = package_digest(package_dir, payload_rel)?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| ErrorCode::SchemaInvalid)?;
    mac.update(&digest);
    Ok(hex_encode(&mac.finalize().into_bytes()))
}

/// Write `signature.hmac` beside the package.
///
/// # Errors
/// Sign or I/O failure.
pub fn write_signature(package_dir: &Path, payload_rel: &str, key: &[u8]) -> Result<(), ErrorCode> {
    let sig = sign_package(package_dir, payload_rel, key)?;
    fs::write(package_dir.join(SIGNATURE_FILE), format!("{sig}\n"))
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    Ok(())
}

/// If `signature.hmac` is present, verify it; if absent, succeed (unsigned allowed).
///
/// # Errors
/// [`ErrorCode::ModuleIncompatible`] when signature present but invalid.
pub fn verify_signature_if_present(
    package_dir: &Path,
    payload_rel: &str,
    key: &[u8],
) -> Result<VerifyStatus, ErrorCode> {
    let path = package_dir.join(SIGNATURE_FILE);
    if !path.exists() {
        return Ok(VerifyStatus::Unsigned);
    }
    let raw = fs::read_to_string(&path).map_err(|_| ErrorCode::SchemaInvalid)?;
    let expected = sign_package(package_dir, payload_rel, key)?;
    let got = raw.trim();
    if got.eq_ignore_ascii_case(&expected) {
        Ok(VerifyStatus::Valid)
    } else {
        Err(ErrorCode::ModuleIncompatible)
    }
}

/// Require a valid signature (curated/core style).
///
/// # Errors
/// Missing or bad signature → incompatible.
pub fn verify_signature_required(
    package_dir: &Path,
    payload_rel: &str,
    key: &[u8],
) -> Result<(), ErrorCode> {
    match verify_signature_if_present(package_dir, payload_rel, key)? {
        VerifyStatus::Valid => Ok(()),
        VerifyStatus::Unsigned => Err(ErrorCode::ModuleIncompatible),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyStatus {
    Unsigned,
    Valid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sign_and_verify() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dir = tmp.path();
        fs::write(
            dir.join("manifest.toml"),
            r#"
id = "t"
version = "0.1.0"
api_version = "sak.v0"
origin = "community"
runtime = "wasm"
payload = "module.wat"
"#,
        )
        .unwrap();
        fs::write(dir.join("module.wat"), b"(module)").unwrap();
        write_signature(dir, "module.wat", SMOKE_HMAC_KEY).unwrap();
        assert_eq!(
            verify_signature_if_present(dir, "module.wat", SMOKE_HMAC_KEY).unwrap(),
            VerifyStatus::Valid
        );
        fs::write(dir.join(SIGNATURE_FILE), "deadbeef\n").unwrap();
        assert_eq!(
            verify_signature_if_present(dir, "module.wat", SMOKE_HMAC_KEY),
            Err(ErrorCode::ModuleIncompatible)
        );
    }
}
