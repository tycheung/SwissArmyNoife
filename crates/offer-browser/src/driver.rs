//! Browser driver port (stub or Playwright process sidecar).

use std::path::Path;

use serde_json::Value;
use types::ErrorCode;

/// Which concrete driver is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserBackendKind {
    Stub,
    Playwright,
}

impl BrowserBackendKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stub => "stub",
            Self::Playwright => "playwright",
        }
    }
}

/// Out-of-process (or in-process stub) browser driver for one binding session.
pub trait BrowserBackend: Send + Sync {
    fn kind(&self) -> BrowserBackendKind;

    /// JSON-lines protocol op against a profile directory.
    fn call(
        &self,
        profile_dir: &Path,
        op: &str,
        params: Value,
    ) -> impl std::future::Future<Output = Result<Value, ErrorCode>> + Send;
}
