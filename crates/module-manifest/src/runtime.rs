//! Runtime kind for module payloads (`sak350` / wasm-first).

use serde::{Deserialize, Serialize};

/// How the package payload is executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Wasm,
    Process,
    /// First-party only — rejected for marketplace install in OSS default (`sak362`).
    Native,
}

impl RuntimeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wasm => "wasm",
            Self::Process => "process",
            Self::Native => "native",
        }
    }

    /// Marketplace OSS default allows wasm (and process); never untrusted native.
    #[must_use]
    pub const fn allowed_for_marketplace_oss(self) -> bool {
        matches!(self, Self::Wasm | Self::Process)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_allowed_native_denied() {
        assert!(RuntimeKind::Wasm.allowed_for_marketplace_oss());
        assert!(!RuntimeKind::Native.allowed_for_marketplace_oss());
    }
}
