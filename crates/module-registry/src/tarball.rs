//! Offline tarball install (`sak365`).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;
use types::ErrorCode;

use crate::store::install_from_path;
use crate::InstalledModule;

/// Install from a `.tar.gz` (or `.tgz`) package archive.
///
/// Archive must contain `manifest.toml` at the root or one level down.
///
/// # Errors
/// Unpack / validation failures.
pub fn install_from_tarball(archive: &Path) -> Result<InstalledModule, ErrorCode> {
    let tmp = tempfile::tempdir().map_err(|_| ErrorCode::SchemaInvalid)?;
    unpack_tarball(archive, tmp.path())?;
    let root = find_package_root(tmp.path())?;
    install_from_path(&root)
}

fn unpack_tarball(archive: &Path, dest: &Path) -> Result<(), ErrorCode> {
    let file = fs::File::open(archive).map_err(|_| ErrorCode::SchemaInvalid)?;
    // `name` is lowercased; ends_with is intentional for `.tar.gz` compound suffix.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    let kind = {
        let name = archive
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            "gz"
        } else if name.ends_with(".tar") {
            "tar"
        } else {
            "guess"
        }
    };
    match kind {
        "gz" => {
            let dec = GzDecoder::new(file);
            let mut ar = Archive::new(dec);
            ar.unpack(dest).map_err(|_| ErrorCode::SchemaInvalid)?;
        }
        "tar" => {
            let mut ar = Archive::new(file);
            ar.unpack(dest).map_err(|_| ErrorCode::SchemaInvalid)?;
        }
        _ => {
            let mut bytes = Vec::new();
            fs::File::open(archive)
                .and_then(|mut f| f.read_to_end(&mut bytes))
                .map_err(|_| ErrorCode::SchemaInvalid)?;
            let dec = GzDecoder::new(bytes.as_slice());
            let mut ar = Archive::new(dec);
            if ar.unpack(dest).is_err() {
                let mut ar = Archive::new(bytes.as_slice());
                ar.unpack(dest).map_err(|_| ErrorCode::SchemaInvalid)?;
            }
        }
    }
    Ok(())
}

fn find_package_root(dir: &Path) -> Result<PathBuf, ErrorCode> {
    if dir.join("manifest.toml").is_file() {
        return Ok(dir.to_path_buf());
    }
    for entry in fs::read_dir(dir).map_err(|_| ErrorCode::SchemaInvalid)? {
        let entry = entry.map_err(|_| ErrorCode::SchemaInvalid)?;
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            let child = entry.path();
            if child.join("manifest.toml").is_file() {
                return Ok(child);
            }
        }
    }
    Err(ErrorCode::SchemaInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::lock_env;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;

    #[test]
    fn tarball_install() {
        let _g = lock_env();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CONFIG_DIR", tmp.path().join("cfg"));
        let pkg = tmp.path().join("pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("manifest.toml"),
            r#"
id = "community.tar"
version = "0.1.0"
api_version = "sak.v0"
origin = "community"
runtime = "wasm"
payload = "module.wat"
"#,
        )
        .unwrap();
        fs::write(
            pkg.join("module.wat"),
            include_str!("../../../modules/community.echo/module.wat"),
        )
        .unwrap();
        let tar_path = tmp.path().join("pkg.tar.gz");
        {
            let file = fs::File::create(&tar_path).unwrap();
            let enc = GzEncoder::new(file, Compression::default());
            let mut builder = Builder::new(enc);
            builder.append_dir_all("community.tar", &pkg).unwrap();
            builder.finish().unwrap();
        }
        let installed = install_from_tarball(&tar_path).expect("install");
        assert_eq!(installed.manifest.id, "community.tar");
        std::env::remove_var("CONFIG_DIR");
    }
}
