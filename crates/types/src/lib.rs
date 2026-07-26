//! Freestanding wire types for `SwissArmyNoife` (no async runtime).

mod fixture;
mod invoke;
mod schema;

pub use fixture::load_offer_fixture;
pub use invoke::{InvokeId, InvokeReq, InvokeResp};
pub use schema::core_schema_document;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Catalog offer identifier (e.g. `llm.chat`, `sandbox.exec`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct OfferId(String);

impl OfferId {
    /// Create an offer id from a non-empty string.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when `raw` is empty or whitespace-only.
    pub fn new(raw: impl Into<String>) -> Result<Self, ErrorCode> {
        let s = raw.into();
        if s.trim().is_empty() {
            return Err(ErrorCode::SchemaInvalid);
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OfferId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque binding handle returned by `bind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct BindingId(Uuid);

impl BindingId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for BindingId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BindingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Stable wire error codes (normative plan §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    #[serde(rename = "policy.denied")]
    PolicyDenied,
    #[serde(rename = "budget.exhausted")]
    BudgetExhausted,
    #[serde(rename = "vault.missing")]
    VaultMissing,
    #[serde(rename = "provider.unreachable")]
    ProviderUnreachable,
    #[serde(rename = "sandbox.violation")]
    SandboxViolation,
    #[serde(rename = "egress.denied")]
    EgressDenied,
    #[serde(rename = "offer.not_found")]
    OfferNotFound,
    #[serde(rename = "binding.expired")]
    BindingExpired,
    #[serde(rename = "module.incompatible")]
    ModuleIncompatible,
    #[serde(rename = "schema.invalid")]
    SchemaInvalid,
}

impl ErrorCode {
    /// Canonical dotted string used on the wire and in MCP errors.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PolicyDenied => "policy.denied",
            Self::BudgetExhausted => "budget.exhausted",
            Self::VaultMissing => "vault.missing",
            Self::ProviderUnreachable => "provider.unreachable",
            Self::SandboxViolation => "sandbox.violation",
            Self::EgressDenied => "egress.denied",
            Self::OfferNotFound => "offer.not_found",
            Self::BindingExpired => "binding.expired",
            Self::ModuleIncompatible => "module.incompatible",
            Self::SchemaInvalid => "schema.invalid",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for ErrorCode {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_id_rejects_empty() {
        assert_eq!(OfferId::new(""), Err(ErrorCode::SchemaInvalid));
        assert_eq!(OfferId::new("   "), Err(ErrorCode::SchemaInvalid));
    }

    #[test]
    fn offer_id_serde_roundtrip() {
        let id = OfferId::new("llm.chat").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"llm.chat\"");
        let back: OfferId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn binding_id_serde_roundtrip() {
        let id = BindingId::from_uuid(Uuid::nil());
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000000\"");
        let back: BindingId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn error_code_wire_strings_match_plan() {
        let expected = [
            (ErrorCode::PolicyDenied, "policy.denied"),
            (ErrorCode::BudgetExhausted, "budget.exhausted"),
            (ErrorCode::VaultMissing, "vault.missing"),
            (ErrorCode::ProviderUnreachable, "provider.unreachable"),
            (ErrorCode::SandboxViolation, "sandbox.violation"),
            (ErrorCode::EgressDenied, "egress.denied"),
            (ErrorCode::OfferNotFound, "offer.not_found"),
            (ErrorCode::BindingExpired, "binding.expired"),
            (ErrorCode::ModuleIncompatible, "module.incompatible"),
            (ErrorCode::SchemaInvalid, "schema.invalid"),
        ];
        for (code, wire) in expected {
            assert_eq!(code.as_str(), wire);
            let json = serde_json::to_string(&code).expect("serialize");
            assert_eq!(json, format!("\"{wire}\""));
            let back: ErrorCode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, code);
        }
    }
}
