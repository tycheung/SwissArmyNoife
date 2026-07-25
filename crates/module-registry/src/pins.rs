//! Consumer `modules.toml` pin file (`sak352`).

use std::fs;
use std::path::PathBuf;

use module_manifest::OriginTier;
use serde::{Deserialize, Serialize};
use types::ErrorCode;

/// One pinned module reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModulePin {
    pub id: String,
    pub version: String,
    pub origin: OriginTier,
    /// `registry` | `tarball` | `path`
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "path".into()
}

/// Root of `modules.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModulesFile {
    #[serde(default)]
    pub module: Vec<ModulePin>,
}

impl ModulesFile {
    /// Replace or append a pin by id.
    pub fn upsert(&mut self, pin: ModulePin) {
        if let Some(existing) = self.module.iter_mut().find(|p| p.id == pin.id) {
            *existing = pin;
        } else {
            self.module.push(pin);
        }
    }
}

/// `{config_dir}/modules.toml`
#[must_use]
pub fn pins_path() -> PathBuf {
    env::config_dir().join("modules.toml")
}

/// Load pins; missing file → empty.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on parse/I/O of an existing file.
pub fn load_pins() -> Result<ModulesFile, ErrorCode> {
    let path = pins_path();
    if !path.exists() {
        return Ok(ModulesFile::default());
    }
    let raw = fs::read_to_string(&path).map_err(|_| ErrorCode::SchemaInvalid)?;
    toml::from_str(&raw).map_err(|_| ErrorCode::SchemaInvalid)
}

/// Persist pins atomically-ish (write then replace).
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] on I/O.
pub fn save_pins(file: &ModulesFile) -> Result<(), ErrorCode> {
    let path = pins_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ErrorCode::SchemaInvalid)?;
    }
    let raw = toml::to_string_pretty(file).map_err(|_| ErrorCode::SchemaInvalid)?;
    fs::write(&path, raw).map_err(|_| ErrorCode::SchemaInvalid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::lock_env;

    #[test]
    fn pins_roundtrip() {
        let _g = lock_env();
        let tmp = tempfile::tempdir().expect("tmp");
        std::env::set_var("CONFIG_DIR", tmp.path());
        let mut file = ModulesFile::default();
        file.upsert(ModulePin {
            id: "community.echo".into(),
            version: "0.1.0".into(),
            origin: OriginTier::Community,
            source: "path".into(),
        });
        save_pins(&file).expect("save");
        let raw = std::fs::read_to_string(pins_path()).expect("read");
        assert!(raw.contains("community.echo"), "raw={raw}");
        let loaded = load_pins().expect("load");
        assert_eq!(loaded.module.len(), 1);
        assert_eq!(loaded.module[0].id, "community.echo");
        std::env::remove_var("CONFIG_DIR");
    }
}
