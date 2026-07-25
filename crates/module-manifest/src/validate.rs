//! Manifest validation (`sak350`, `sak362`).

use types::ErrorCode;

use crate::{ModuleManifest, RuntimeKind};

/// Validate required fields and OSS marketplace runtime rules.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] for missing fields;
/// [`ErrorCode::ModuleIncompatible`] when `runtime = native` in marketplace packages.
pub fn validate_manifest(m: &ModuleManifest) -> Result<(), ErrorCode> {
    if m.id.trim().is_empty() || m.version.trim().is_empty() || m.api_version.trim().is_empty() {
        return Err(ErrorCode::SchemaInvalid);
    }
    if m.payload.trim().is_empty() {
        return Err(ErrorCode::SchemaInvalid);
    }
    if m.runtime == RuntimeKind::Native {
        // sak362: deny native dylib path in OSS marketplace default
        return Err(ErrorCode::ModuleIncompatible);
    }
    if !m.runtime.allowed_for_marketplace_oss() {
        return Err(ErrorCode::ModuleIncompatible);
    }
    if let Some(slot) = m.enterprise_slot {
        if !slot.allowed_for_marketplace_oss() {
            return Err(ErrorCode::ModuleIncompatible);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EnterpriseRuntimeSlot, OriginTier, RuntimeKind};

    fn base() -> ModuleManifest {
        ModuleManifest {
            id: "x".into(),
            version: "0.1.0".into(),
            api_version: "sak.v0".into(),
            origin: OriginTier::Community,
            runtime: RuntimeKind::Wasm,
            min_broker_version: "0.1.0".into(),
            permissions: vec![],
            payload: "module.wasm".into(),
            enterprise_slot: None,
        }
    }

    #[test]
    fn rejects_native_runtime() {
        let mut m = base();
        m.runtime = RuntimeKind::Native;
        assert_eq!(validate_manifest(&m), Err(ErrorCode::ModuleIncompatible));
    }

    #[test]
    fn accepts_wasm() {
        assert!(validate_manifest(&base()).is_ok());
    }

    #[test]
    fn rejects_enterprise_k8s_slot() {
        let mut m = base();
        m.enterprise_slot = Some(EnterpriseRuntimeSlot::K8s);
        assert_eq!(validate_manifest(&m), Err(ErrorCode::ModuleIncompatible));
    }

    #[test]
    fn rejects_enterprise_e2b_slot() {
        let mut m = base();
        m.enterprise_slot = Some(EnterpriseRuntimeSlot::E2b);
        assert_eq!(validate_manifest(&m), Err(ErrorCode::ModuleIncompatible));
    }
}
