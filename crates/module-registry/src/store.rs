//! On-disk module cache under `{CONFIG_DIR}/modules/` (`sak351`).

use std::fs;
use std::path::{Path, PathBuf};

use module_manifest::ModuleManifest;
use types::ErrorCode;

/// One installed module package on disk.
#[derive(Clone, Debug)]
pub struct InstalledModule {
    pub root: PathBuf,
    pub manifest: ModuleManifest,
}

/// `{config_dir}/modules`
#[must_use]
pub fn module_cache_dir() -> PathBuf {
    env::config_dir().join("modules")
}

/// Install by copying a package directory that contains `manifest.toml`.
///
/// Verifies Ed25519 / HMAC when present; core/curated require a valid signature
/// (`sak357` / `sak357-c`).
///
/// # Errors
/// Missing manifest, validation failure, bad signature, or I/O.
pub fn install_from_path(package_dir: &Path) -> Result<InstalledModule, ErrorCode> {
    let manifest_path = package_dir.join("manifest.toml");
    let raw = fs::read_to_string(&manifest_path).map_err(|_| ErrorCode::SchemaInvalid)?;
    let manifest = ModuleManifest::parse_and_validate(&raw)?;
    crate::verify_package(package_dir, &manifest.payload, manifest.origin)?;
    let dest = module_cache_dir()
        .join(&manifest.id)
        .join(&manifest.version);
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|_| ErrorCode::SchemaInvalid)?;
    }
    fs::create_dir_all(&dest).map_err(|_| ErrorCode::SchemaInvalid)?;
    copy_dir(package_dir, &dest)?;
    Ok(InstalledModule {
        root: dest,
        manifest,
    })
}

/// Remove an installed module (`id` + optional `version`; latest/only if version omitted).
///
/// # Errors
/// Not found → [`ErrorCode::OfferNotFound`]; I/O → schema invalid.
pub fn remove_installed(id: &str, version: Option<&str>) -> Result<PathBuf, ErrorCode> {
    let id_root = module_cache_dir().join(id);
    if !id_root.exists() {
        return Err(ErrorCode::OfferNotFound);
    }
    let target = if let Some(v) = version {
        let p = id_root.join(v);
        if !p.exists() {
            return Err(ErrorCode::OfferNotFound);
        }
        p
    } else {
        let mut versions: Vec<_> = fs::read_dir(&id_root)
            .map_err(|_| ErrorCode::SchemaInvalid)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        if versions.is_empty() {
            return Err(ErrorCode::OfferNotFound);
        }
        versions.sort();
        versions.pop().ok_or(ErrorCode::OfferNotFound)?
    };
    fs::remove_dir_all(&target).map_err(|_| ErrorCode::SchemaInvalid)?;
    // drop empty id dir
    if id_root.read_dir().is_ok_and(|mut d| d.next().is_none()) {
        let _ = fs::remove_dir(&id_root);
    }
    Ok(target)
}

/// Find one installed module by id (highest version if several).
///
/// # Errors
/// [`ErrorCode::OfferNotFound`] when missing.
pub fn get_installed(id: &str, version: Option<&str>) -> Result<InstalledModule, ErrorCode> {
    let mut matches: Vec<_> = list_installed()?
        .into_iter()
        .filter(|m| m.manifest.id == id)
        .collect();
    if let Some(v) = version {
        matches.retain(|m| m.manifest.version == v);
    }
    matches.sort_by(|a, b| a.manifest.version.cmp(&b.manifest.version));
    matches.pop().ok_or(ErrorCode::OfferNotFound)
}

/// List installed module dirs that still have a valid manifest.
///
/// # Errors
/// I/O when scanning the cache root.
pub fn list_installed() -> Result<Vec<InstalledModule>, ErrorCode> {
    let root = module_cache_dir();
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for id_entry in fs::read_dir(&root).map_err(|_| ErrorCode::SchemaInvalid)? {
        let id_entry = id_entry.map_err(|_| ErrorCode::SchemaInvalid)?;
        if !id_entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        for ver_entry in fs::read_dir(id_entry.path()).map_err(|_| ErrorCode::SchemaInvalid)? {
            let ver_entry = ver_entry.map_err(|_| ErrorCode::SchemaInvalid)?;
            if !ver_entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let manifest_path = ver_entry.path().join("manifest.toml");
            let Ok(raw) = fs::read_to_string(&manifest_path) else {
                continue;
            };
            let Ok(manifest) = ModuleManifest::parse_and_validate(&raw) else {
                continue;
            };
            out.push(InstalledModule {
                root: ver_entry.path(),
                manifest,
            });
        }
    }
    out.sort_by(|a, b| {
        a.manifest
            .id
            .cmp(&b.manifest.id)
            .then(a.manifest.version.cmp(&b.manifest.version))
    });
    Ok(out)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), ErrorCode> {
    for entry in fs::read_dir(src).map_err(|_| ErrorCode::SchemaInvalid)? {
        let entry = entry.map_err(|_| ErrorCode::SchemaInvalid)?;
        let ty = entry.file_type().map_err(|_| ErrorCode::SchemaInvalid)?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            fs::create_dir_all(&to).map_err(|_| ErrorCode::SchemaInvalid)?;
            copy_dir(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &to).map_err(|_| ErrorCode::SchemaInvalid)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::lock_env;

    #[test]
    fn install_list_roundtrip() {
        let _g = lock_env();
        let tmp = tempfile::tempdir().expect("tmp");
        std::env::set_var("CONFIG_DIR", tmp.path());
        let pkg = tmp.path().join("pkg");
        fs::create_dir_all(&pkg).expect("mkdir");
        fs::write(
            pkg.join("manifest.toml"),
            r#"
id = "community.echo"
version = "0.1.0"
api_version = "sak.v0"
origin = "community"
runtime = "wasm"
payload = "module.wasm"
"#,
        )
        .expect("manifest");
        fs::write(pkg.join("module.wasm"), b"\0asm").expect("wasm");
        let installed = install_from_path(&pkg).expect("install");
        assert_eq!(installed.manifest.id, "community.echo");
        let listed = list_installed().expect("list");
        assert_eq!(listed.len(), 1);
        std::env::remove_var("CONFIG_DIR");
    }
}
