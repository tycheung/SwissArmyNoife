//! Process-wide wasm handle cache with hot-reload (`sak360-b`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use runtime_wasm::WasmHandle;
use types::ErrorCode;

/// Cached handles keyed by absolute payload path.
#[derive(Debug, Default)]
pub struct ModuleRuntime {
    handles: Mutex<HashMap<PathBuf, WasmHandle>>,
}

impl ModuleRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load (or reuse) a handle, reloading from disk when the fingerprint changes.
    ///
    /// # Errors
    /// Load / ABI / lock errors.
    pub fn invoke_add(&self, payload: &Path, a: i32, b: i32) -> Result<i32, ErrorCode> {
        let key = payload
            .canonicalize()
            .unwrap_or_else(|_| payload.to_path_buf());
        let mut map = self.handles.lock().map_err(|_| ErrorCode::SchemaInvalid)?;
        if let Some(h) = map.get_mut(&key) {
            let _ = h.reload_if_changed()?;
            return h.call_add(a, b);
        }
        let h = WasmHandle::load(&key)?;
        let sum = h.call_add(a, b)?;
        map.insert(key, h);
        Ok(sum)
    }

    /// Drop cached handle for a path (e.g. after remove).
    pub fn invalidate(&self, payload: &Path) {
        let key = payload
            .canonicalize()
            .unwrap_or_else(|_| payload.to_path_buf());
        if let Ok(mut map) = self.handles.lock() {
            map.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_wasm::SMOKE_ADD_WAT;
    use std::fs;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn cache_reload() {
        let _g = LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("m.wat");
        fs::write(&path, SMOKE_ADD_WAT).unwrap();
        let rt = ModuleRuntime::new();
        assert_eq!(rt.invoke_add(&path, 1, 2).unwrap(), 3);
        assert_eq!(rt.invoke_add(&path, 2, 2).unwrap(), 4);
        let mut wat = SMOKE_ADD_WAT.to_owned();
        wat.push_str("\n;; x\n");
        fs::write(&path, wat).unwrap();
        assert_eq!(rt.invoke_add(&path, 5, 5).unwrap(), 10);
    }
}
