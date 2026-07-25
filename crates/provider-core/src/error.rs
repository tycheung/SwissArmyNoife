//! Provider-local errors mapped to broker [`ErrorCode`](types::ErrorCode) where useful.

use thiserror::Error;
use types::ErrorCode;

/// Failures from a concrete LLM provider adapter.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider unreachable: {0}")]
    Unreachable(String),
    #[error("schema invalid: {0}")]
    SchemaInvalid(String),
    #[error("provider: {0}")]
    Other(String),
}

impl ProviderError {
    /// Map to a stable broker wire code for invoke responses.
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::SchemaInvalid(_) => ErrorCode::SchemaInvalid,
            Self::Unreachable(_) | Self::Other(_) => ErrorCode::ProviderUnreachable,
        }
    }
}
