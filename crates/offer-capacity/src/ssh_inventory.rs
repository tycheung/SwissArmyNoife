//! SSH inventory text helpers (`sak275-g` / `sak275-h` / `sak275-i`).
//!
//! Line-oriented parse of the YAML sketch in `docs/ssh-fleet-probe-followup.md`.
//! File load reads UTF-8 only — **no SSH**.

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Env path to an SSH inventory YAML sketch (`sak275-j`).
pub const SSH_INVENTORY_ENV: &str = "CAPACITY_SSH_INVENTORY";

/// Extract host ids from an inventory YAML sketch (`sak275-g`).
///
/// Matches list items like `- id: gpu-a` (optional quotes). Ignores `defaults:`
/// and other keys. Duplicate ids are returned as-is (caller may reject later).
#[must_use]
pub fn host_ids_from_inventory_sketch(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("- id:") else {
            continue;
        };
        let id = rest.trim().trim_matches('"').trim_matches('\'').trim();
        if !id.is_empty() {
            ids.push(id.to_string());
        }
    }
    ids
}

/// Count host entries in an inventory sketch (`sak275-g`).
#[must_use]
pub fn count_hosts_in_inventory_sketch(text: &str) -> usize {
    host_ids_from_inventory_sketch(text).len()
}

/// Errors from inventory load / validation (`sak275-h` / `sak275-i`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryLoadError {
    Io(String),
    DuplicateId(String),
}

impl fmt::Display for InventoryLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "ssh inventory load failed: {msg}"),
            Self::DuplicateId(id) => write!(f, "ssh inventory duplicate host id: {id}"),
        }
    }
}

impl std::error::Error for InventoryLoadError {}

/// Reject duplicate host ids (`sak275-i`).
///
/// # Errors
/// [`InventoryLoadError::DuplicateId`] when an id appears more than once.
pub fn unique_host_ids(ids: &[String]) -> Result<Vec<String>, InventoryLoadError> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if !seen.insert(id.clone()) {
            return Err(InventoryLoadError::DuplicateId(id.clone()));
        }
        out.push(id.clone());
    }
    Ok(out)
}

/// Read a UTF-8 inventory file and extract host ids (`sak275-h`).
///
/// Does **not** open SSH or validate addresses — parse only.
///
/// # Errors
/// [`InventoryLoadError::Io`] when the file cannot be read.
pub fn host_ids_from_inventory_path(path: &Path) -> Result<Vec<String>, InventoryLoadError> {
    let text = fs::read_to_string(path).map_err(|e| InventoryLoadError::Io(e.to_string()))?;
    Ok(host_ids_from_inventory_sketch(&text))
}

/// Load path then require unique ids (`sak275-i`).
///
/// # Errors
/// Propagates I/O errors from [`host_ids_from_inventory_path`] or duplicate-id errors.
pub fn unique_host_ids_from_inventory_path(path: &Path) -> Result<Vec<String>, InventoryLoadError> {
    let ids = host_ids_from_inventory_path(path)?;
    unique_host_ids(&ids)
}

/// Resolve inventory path from [`SSH_INVENTORY_ENV`] (`sak275-j`).
///
/// Empty / unset → `None`. Does not read the file.
#[must_use]
pub fn inventory_path_from_env() -> Option<PathBuf> {
    std::env::var(SSH_INVENTORY_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Load unique host ids when [`SSH_INVENTORY_ENV`] is set (`sak275-j`).
///
/// Returns `Ok(None)` when the env var is unset/blank.
///
/// # Errors
/// Propagates inventory load / uniqueness errors when the env path is set.
pub fn unique_host_ids_from_env() -> Result<Option<Vec<String>>, InventoryLoadError> {
    match inventory_path_from_env() {
        Some(path) => unique_host_ids_from_inventory_path(&path).map(Some),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SKETCH: &str = r#"
version: 1
defaults:
  connect_timeout_secs: 10
hosts:
  - id: gpu-a
    address: "gpu-a.internal:22"
  - id: cpu-b
    address: "10.0.0.12:22"
"#;

    #[test]
    fn counts_two_hosts() {
        assert_eq!(count_hosts_in_inventory_sketch(SKETCH), 2);
        assert_eq!(
            host_ids_from_inventory_sketch(SKETCH),
            vec!["gpu-a".to_string(), "cpu-b".to_string()]
        );
    }

    #[test]
    fn empty_hosts() {
        assert_eq!(count_hosts_in_inventory_sketch("hosts:\n"), 0);
    }

    #[test]
    fn load_from_path() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("sak275-h-inventory-{}.yaml", std::process::id()));
        {
            let mut f = fs::File::create(&path).expect("create");
            f.write_all(SKETCH.as_bytes()).expect("write");
        }
        let ids = host_ids_from_inventory_path(&path).expect("load");
        let _ = fs::remove_file(&path);
        assert_eq!(ids, vec!["gpu-a".to_string(), "cpu-b".to_string()]);
    }

    #[test]
    fn load_missing_path() {
        let path = Path::new("/nonexistent/sak275-h-inventory.yaml");
        assert!(matches!(
            host_ids_from_inventory_path(path),
            Err(InventoryLoadError::Io(_))
        ));
    }

    #[test]
    fn unique_ok() {
        let ids = vec!["a".into(), "b".into()];
        assert_eq!(unique_host_ids(&ids).unwrap(), ids);
    }

    #[test]
    fn unique_rejects_duplicate() {
        let ids = vec!["a".into(), "b".into(), "a".into()];
        assert_eq!(
            unique_host_ids(&ids).unwrap_err(),
            InventoryLoadError::DuplicateId("a".into())
        );
    }

    #[test]
    fn env_path_unset_is_none() {
        let prior = std::env::var(SSH_INVENTORY_ENV).ok();
        std::env::remove_var(SSH_INVENTORY_ENV);
        assert!(inventory_path_from_env().is_none());
        assert_eq!(unique_host_ids_from_env().unwrap(), None);
        match prior {
            Some(v) => std::env::set_var(SSH_INVENTORY_ENV, v),
            None => std::env::remove_var(SSH_INVENTORY_ENV),
        }
    }

    #[test]
    fn env_path_loads_unique() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("sak275-j-inventory-{}.yaml", std::process::id()));
        {
            let mut f = fs::File::create(&path).expect("create");
            f.write_all(SKETCH.as_bytes()).expect("write");
        }
        let prior = std::env::var(SSH_INVENTORY_ENV).ok();
        std::env::set_var(SSH_INVENTORY_ENV, &path);
        let ids = unique_host_ids_from_env().expect("load").expect("some");
        match prior {
            Some(v) => std::env::set_var(SSH_INVENTORY_ENV, v),
            None => std::env::remove_var(SSH_INVENTORY_ENV),
        }
        let _ = fs::remove_file(&path);
        assert_eq!(ids, vec!["gpu-a".to_string(), "cpu-b".to_string()]);
    }
}
