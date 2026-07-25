//! Enterprise-only runtime slots (k8s / e2b) — not OSS marketplace install targets.

use serde::{Deserialize, Serialize};

/// Optional enterprise execution slot (hosted runtime outside local wasm/process).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnterpriseRuntimeSlot {
    K8s,
    E2b,
}

impl EnterpriseRuntimeSlot {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::K8s => "k8s",
            Self::E2b => "e2b",
        }
    }

    /// OSS marketplace default rejects enterprise slots as install targets.
    #[must_use]
    pub const fn allowed_for_marketplace_oss(self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_slots_denied_for_oss_marketplace() {
        assert!(!EnterpriseRuntimeSlot::K8s.allowed_for_marketplace_oss());
        assert!(!EnterpriseRuntimeSlot::E2b.allowed_for_marketplace_oss());
    }

    #[test]
    fn serde_roundtrip() {
        let slot = EnterpriseRuntimeSlot::K8s;
        let v = serde_json::to_value(slot).expect("serialize");
        assert_eq!(v, serde_json::json!("k8s"));
        let back: EnterpriseRuntimeSlot = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, slot);
    }
}
