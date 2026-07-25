//! SDK errors.

use thiserror::Error;
use types::ErrorCode;

/// Client-facing errors (map toward [`ErrorCode`] when possible).
#[derive(Debug, Error)]
pub enum SdkError {
    #[error("http: {0}")]
    Http(String),
    #[error("schema: {0}")]
    Schema(String),
    #[error("{0}")]
    Broker(ErrorCode),
}

impl SdkError {
    #[must_use]
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::Broker(c) => *c,
            Self::Schema(_) => ErrorCode::SchemaInvalid,
            Self::Http(_) => ErrorCode::ProviderUnreachable,
        }
    }
}
