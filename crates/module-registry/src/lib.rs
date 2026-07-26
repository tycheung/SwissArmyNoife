//! `SwissArmyNoife` local module cache + pin file (`sak351` / `sak352`).

mod ed25519_sig;
mod pins;
mod registry_client;
mod runtime_cache;
mod store;
mod tarball;
mod verify;

#[cfg(test)]
mod test_env;

pub use ed25519_sig::{
    generate_signing_key, sign_ed25519, verify_ed25519_if_present, write_ed25519_signature,
    ED25519_PUBKEY_FILE, ED25519_SIGNATURE_FILE,
};
pub use pins::{load_pins, pins_path, save_pins, ModulePin, ModulesFile};
pub use registry_client::{
    write_download, FakeRegistryClient, HttpRegistryClient, RegistryClient, ResolvedModule,
};
pub use runtime_cache::ModuleRuntime;
pub use store::{
    get_installed, install_from_path, list_installed, module_cache_dir, remove_installed,
    InstalledModule,
};
pub use tarball::install_from_tarball;
pub use verify::{
    package_digest, sign_package, verify_signature_if_present, verify_signature_required,
    write_signature, VerifyStatus, SIGNATURE_FILE, SMOKE_HMAC_KEY,
};

use std::path::Path;

use module_manifest::OriginTier;
use types::ErrorCode;

/// Prefer Ed25519 when `signature.ed25519` exists; else HMAC; enforce curated/core.
///
/// # Errors
/// Bad / missing-required signature → [`ErrorCode::ModuleIncompatible`].
pub fn verify_package(
    package_dir: &Path,
    payload_rel: &str,
    origin: OriginTier,
) -> Result<VerifyStatus, ErrorCode> {
    let status = if package_dir.join(ED25519_SIGNATURE_FILE).exists() {
        verify_ed25519_if_present(package_dir, payload_rel, None)?
    } else {
        verify_signature_if_present(package_dir, payload_rel, SMOKE_HMAC_KEY)?
    };
    if origin.requires_signature() && status == VerifyStatus::Unsigned {
        return Err(ErrorCode::ModuleIncompatible);
    }
    Ok(status)
}

/// Install from a package directory and upsert a pin.
///
/// # Errors
/// Schema / I/O / incompatible module errors.
pub fn install_and_pin(package_dir: &Path, source: &str) -> Result<InstalledModule, ErrorCode> {
    let installed = install_from_path(package_dir)?;
    pin_installed(&installed, source)?;
    Ok(installed)
}

/// Install from `.tar.gz` / `.tgz` and pin (`source = tarball`).
///
/// # Errors
/// Unpack / validate / pin errors.
pub fn install_tarball_and_pin(archive: &Path) -> Result<InstalledModule, ErrorCode> {
    let installed = install_from_tarball(archive)?;
    pin_installed(&installed, "tarball")?;
    Ok(installed)
}

/// Resolve + download from a registry, then install the tarball (`sak364-b`).
///
/// # Errors
/// Resolve / download / unpack / pin errors.
pub async fn install_from_registry(
    client: &impl RegistryClient,
    id: &str,
    version: &str,
) -> Result<InstalledModule, ErrorCode> {
    let resolved = client.resolve(id, version).await?;
    let bytes = client.download(&resolved.download_url).await?;
    if let Some(expected) = &resolved.sha256 {
        let got = {
            use sha2::{Digest, Sha256};
            hex_encode_sha(&Sha256::digest(&bytes))
        };
        if !got.eq_ignore_ascii_case(expected.trim()) {
            return Err(ErrorCode::ModuleIncompatible);
        }
    }
    let tmp = tempfile::tempdir().map_err(|_| ErrorCode::SchemaInvalid)?;
    let archive = tmp.path().join("pkg.tgz");
    write_download(&archive, &bytes)?;
    let installed = install_from_tarball(&archive)?;
    pin_installed(&installed, &format!("registry:{}", resolved.download_url))?;
    Ok(installed)
}

fn hex_encode_sha(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Re-install from path (update) and refresh pin.
///
/// # Errors
/// Same as [`install_and_pin`].
pub fn update_from_path(package_dir: &Path) -> Result<InstalledModule, ErrorCode> {
    install_and_pin(package_dir, "path")
}

/// Remove installed module and drop pin entry.
///
/// # Errors
/// Not found / I/O.
pub fn remove_and_unpin(id: &str, version: Option<&str>) -> Result<(), ErrorCode> {
    remove_installed(id, version)?;
    let mut file = load_pins().unwrap_or_default();
    file.module.retain(|p| {
        if p.id != id {
            return true;
        }
        match version {
            Some(v) => p.version != v,
            None => false,
        }
    });
    save_pins(&file)?;
    Ok(())
}

fn pin_installed(installed: &InstalledModule, source: &str) -> Result<(), ErrorCode> {
    let mut file = load_pins().unwrap_or_default();
    file.upsert(ModulePin {
        id: installed.manifest.id.clone(),
        version: installed.manifest.version.clone(),
        origin: installed.manifest.origin,
        source: source.to_owned(),
    });
    save_pins(&file)
}

#[cfg(test)]
mod curated_sig_tests {
    use super::*;
    use std::fs;

    use crate::test_env::lock_env;

    #[test]
    fn curated_unsigned_rejected() {
        let _g = lock_env();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CONFIG_DIR", tmp.path());
        let pkg = tmp.path().join("pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("manifest.toml"),
            r#"
id = "curated.demo"
version = "0.1.0"
api_version = "sak.v0"
origin = "curated"
runtime = "wasm"
payload = "module.wat"
"#,
        )
        .unwrap();
        fs::write(pkg.join("module.wat"), b"(module)").unwrap();
        assert_eq!(
            install_from_path(&pkg).err(),
            Some(ErrorCode::ModuleIncompatible)
        );
    }

    #[test]
    fn curated_hmac_accepted() {
        let _g = lock_env();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CONFIG_DIR", tmp.path());
        let pkg = tmp.path().join("pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("manifest.toml"),
            r#"
id = "curated.demo"
version = "0.1.0"
api_version = "sak.v0"
origin = "curated"
runtime = "wasm"
payload = "module.wat"
"#,
        )
        .unwrap();
        fs::write(pkg.join("module.wat"), b"(module)").unwrap();
        write_signature(&pkg, "module.wat", SMOKE_HMAC_KEY).unwrap();
        let installed = install_from_path(&pkg).unwrap();
        assert_eq!(installed.manifest.id, "curated.demo");
    }
}
