//! Ed25519 package signatures (`sak357-b`).

use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use types::ErrorCode;

use crate::verify::{hex_decode, hex_encode, package_digest, VerifyStatus};

/// Filename for Ed25519 signature (hex of 64-byte sig).
pub const ED25519_SIGNATURE_FILE: &str = "signature.ed25519";

/// Optional verifying key hex file in the package (32-byte pubkey).
pub const ED25519_PUBKEY_FILE: &str = "signing_key.pub";

/// Sign package digest with an Ed25519 signing key; return hex signature.
///
/// # Errors
/// Digest failure.
pub fn sign_ed25519(
    package_dir: &Path,
    payload_rel: &str,
    signing_key: &SigningKey,
) -> Result<String, ErrorCode> {
    let digest = package_digest(package_dir, payload_rel)?;
    let sig = signing_key.sign(&digest);
    Ok(hex_encode(sig.to_bytes().as_ref()))
}

/// Write `signature.ed25519` (+ optional `signing_key.pub`).
///
/// # Errors
/// Sign / I/O.
pub fn write_ed25519_signature(
    package_dir: &Path,
    payload_rel: &str,
    signing_key: &SigningKey,
    write_pubkey: bool,
) -> Result<(), ErrorCode> {
    let sig = sign_ed25519(package_dir, payload_rel, signing_key)?;
    fs::write(package_dir.join(ED25519_SIGNATURE_FILE), format!("{sig}\n"))
        .map_err(|_| ErrorCode::SchemaInvalid)?;
    if write_pubkey {
        let pk = hex_encode(signing_key.verifying_key().as_bytes());
        fs::write(package_dir.join(ED25519_PUBKEY_FILE), format!("{pk}\n"))
            .map_err(|_| ErrorCode::SchemaInvalid)?;
    }
    Ok(())
}

/// Verify Ed25519 signature when file present.
///
/// # Errors
/// Bad signature / key → [`ErrorCode::ModuleIncompatible`].
pub fn verify_ed25519_if_present(
    package_dir: &Path,
    payload_rel: &str,
    trusted_pubkey: Option<&VerifyingKey>,
) -> Result<VerifyStatus, ErrorCode> {
    let sig_path = package_dir.join(ED25519_SIGNATURE_FILE);
    if !sig_path.exists() {
        return Ok(VerifyStatus::Unsigned);
    }
    let vk = match trusted_pubkey {
        Some(k) => *k,
        None => {
            let raw = fs::read_to_string(package_dir.join(ED25519_PUBKEY_FILE))
                .map_err(|_| ErrorCode::ModuleIncompatible)?;
            let bytes = hex_decode(raw.trim())?;
            let arr: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| ErrorCode::ModuleIncompatible)?;
            VerifyingKey::from_bytes(&arr).map_err(|_| ErrorCode::ModuleIncompatible)?
        }
    };
    let sig_hex = fs::read_to_string(&sig_path).map_err(|_| ErrorCode::SchemaInvalid)?;
    let sig_bytes = hex_decode(sig_hex.trim())?;
    let arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    let sig = Signature::from_bytes(&arr);
    let digest = package_digest(package_dir, payload_rel)?;
    vk.verify(&digest, &sig)
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    Ok(VerifyStatus::Valid)
}

/// Generate a random signing key (tests / local tooling).
#[must_use]
pub fn generate_signing_key() -> SigningKey {
    let mut rng = rand::rngs::OsRng;
    SigningKey::generate(&mut rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn ed25519_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(
            dir.join("manifest.toml"),
            r#"
id = "t"
version = "0.1.0"
api_version = "sak.v0"
origin = "curated"
runtime = "wasm"
payload = "module.wat"
"#,
        )
        .unwrap();
        fs::write(dir.join("module.wat"), b"(module)").unwrap();
        let sk = generate_signing_key();
        write_ed25519_signature(dir, "module.wat", &sk, true).unwrap();
        assert_eq!(
            verify_ed25519_if_present(dir, "module.wat", None).unwrap(),
            VerifyStatus::Valid
        );
        fs::write(dir.join(ED25519_SIGNATURE_FILE), "00".repeat(64)).unwrap();
        assert_eq!(
            verify_ed25519_if_present(dir, "module.wat", None),
            Err(ErrorCode::ModuleIncompatible)
        );
    }
}
