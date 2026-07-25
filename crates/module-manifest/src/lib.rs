//! Marketplace `manifest.toml` schema + validation (`sak350`).

mod enterprise;
mod origin;
mod permissions;
mod runtime;
mod validate;

pub use enterprise::EnterpriseRuntimeSlot;
pub use origin::OriginTier;
pub use permissions::{permissions_to_policy_defaults, PermissionDecl};
pub use runtime::RuntimeKind;
pub use validate::validate_manifest;

use serde::{Deserialize, Serialize};
use types::ErrorCode;

/// Parsed module package manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: String,
    pub version: String,
    pub api_version: String,
    pub origin: OriginTier,
    pub runtime: RuntimeKind,
    #[serde(default = "default_min_broker")]
    pub min_broker_version: String,
    #[serde(default)]
    pub permissions: Vec<PermissionDecl>,
    /// Relative payload path inside the package (e.g. `module.wasm`).
    #[serde(default = "default_payload")]
    pub payload: String,
    /// Enterprise-only hosted runtime slot (`k8s` / `e2b`); denied in OSS marketplace default.
    #[serde(default)]
    pub enterprise_slot: Option<EnterpriseRuntimeSlot>,
}

fn default_min_broker() -> String {
    "0.1.0".into()
}

fn default_payload() -> String {
    "module.wasm".into()
}

impl ModuleManifest {
    /// Parse TOML text into a manifest (does not fully validate).
    ///
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] on TOML/serde failure.
    pub fn parse_toml(raw: &str) -> Result<Self, ErrorCode> {
        toml::from_str(raw).map_err(|_| ErrorCode::SchemaInvalid)
    }

    /// Parse and validate.
    ///
    /// # Errors
    /// [`ErrorCode::SchemaInvalid`] or [`ErrorCode::ModuleIncompatible`].
    pub fn parse_and_validate(raw: &str) -> Result<Self, ErrorCode> {
        let m = Self::parse_toml(raw)?;
        validate_manifest(&m)?;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_wasm_manifest() {
        let raw = r#"
id = "community.echo"
version = "0.1.0"
api_version = "sak.v0"
origin = "community"
runtime = "wasm"
payload = "module.wasm"
"#;
        let m = ModuleManifest::parse_and_validate(raw).expect("ok");
        assert_eq!(m.id, "community.echo");
        assert_eq!(m.runtime, RuntimeKind::Wasm);
        assert_eq!(m.origin, OriginTier::Community);
    }

    #[test]
    fn rejects_empty_id() {
        let raw = r#"
id = ""
version = "0.1.0"
api_version = "sak.v0"
origin = "core"
runtime = "wasm"
"#;
        assert_eq!(
            ModuleManifest::parse_and_validate(raw),
            Err(ErrorCode::SchemaInvalid)
        );
    }
}
