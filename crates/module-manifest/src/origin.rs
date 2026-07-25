//! Origin tiers (`sak358`).

use serde::{Deserialize, Serialize};

/// Publisher trust tier (Nimbusware standards mart spirit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OriginTier {
    Core,
    Curated,
    Community,
    Enterprise,
}

impl OriginTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Curated => "curated",
            Self::Community => "community",
            Self::Enterprise => "enterprise",
        }
    }

    /// Default policy tightness hint for the tier.
    #[must_use]
    pub const fn trust_note(self) -> &'static str {
        match self {
            Self::Core | Self::Curated => "full_invoke",
            Self::Community => "warn_tighter_defaults",
            Self::Enterprise => "tenant_policy",
        }
    }

    /// Core/curated packages must carry a valid signature (`sak357-c`).
    #[must_use]
    pub const fn requires_signature(self) -> bool {
        matches!(self, Self::Core | Self::Curated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_notes() {
        assert_eq!(OriginTier::Core.trust_note(), "full_invoke");
        assert_eq!(OriginTier::Community.trust_note(), "warn_tighter_defaults");
        assert!(OriginTier::Curated.requires_signature());
        assert!(!OriginTier::Community.requires_signature());
    }
}
