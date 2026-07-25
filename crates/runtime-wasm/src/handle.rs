//! Loaded wasm module with fingerprint-based hot-reload (`sak360`).

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use types::ErrorCode;

use crate::abi::{abi_version_bytes, call_add_bytes, load_module_bytes};

/// Host ABI major version expected from `sak_abi_version`.
pub const ABI_VERSION: i32 = 1;

/// Fingerprint of raw on-disk bytes (before WAT compile).
#[must_use]
pub fn payload_fingerprint(raw_file_bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(raw_file_bytes).into()
}

/// Cached wasm payload with reload-on-change.
#[derive(Clone, Debug)]
pub struct WasmHandle {
    path: PathBuf,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
}

impl WasmHandle {
    /// Load path, require compatible ABI, cache bytes.
    ///
    /// # Errors
    /// Load / ABI mismatch.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ErrorCode> {
        let path = path.into();
        let raw = std::fs::read(&path).map_err(|_| ErrorCode::ModuleIncompatible)?;
        let fingerprint = payload_fingerprint(&raw);
        let bytes = load_module_bytes(&path)?;
        let ver = abi_version_bytes(&bytes)?;
        if ver != ABI_VERSION {
            return Err(ErrorCode::ModuleIncompatible);
        }
        Ok(Self {
            path,
            bytes,
            fingerprint,
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    /// Reload from disk when raw file fingerprint changes.
    ///
    /// Returns `true` when a reload happened.
    ///
    /// # Errors
    /// Load / ABI errors on changed payload.
    pub fn reload_if_changed(&mut self) -> Result<bool, ErrorCode> {
        let raw = std::fs::read(&self.path).map_err(|_| ErrorCode::ModuleIncompatible)?;
        let fp = payload_fingerprint(&raw);
        if fp == self.fingerprint {
            return Ok(false);
        }
        let bytes = load_module_bytes(&self.path)?;
        let ver = abi_version_bytes(&bytes)?;
        if ver != ABI_VERSION {
            return Err(ErrorCode::ModuleIncompatible);
        }
        self.bytes = bytes;
        self.fingerprint = fp;
        Ok(true)
    }

    /// Call exported `add`.
    ///
    /// # Errors
    /// Instantiation / call failure.
    pub fn call_add(&self, a: i32, b: i32) -> Result<i32, ErrorCode> {
        call_add_bytes(&self.bytes, a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::SMOKE_ADD_WAT;
    use std::fs;

    #[test]
    fn hot_reload_on_change() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("m.wat");
        fs::write(&path, SMOKE_ADD_WAT).unwrap();
        let mut h = WasmHandle::load(&path).unwrap();
        assert!(!h.reload_if_changed().unwrap());
        assert_eq!(h.call_add(1, 1).unwrap(), 2);
        let mut wat = SMOKE_ADD_WAT.to_owned();
        wat.push_str("\n;; touch\n");
        fs::write(&path, wat).unwrap();
        assert!(h.reload_if_changed().unwrap());
        assert_eq!(h.call_add(3, 4).unwrap(), 7);
        fs::write(
            &path,
            r#"(module (func (export "sak_abi_version") (result i32) i32.const 99))"#,
        )
        .unwrap();
        assert_eq!(h.reload_if_changed(), Err(ErrorCode::ModuleIncompatible));
    }
}
