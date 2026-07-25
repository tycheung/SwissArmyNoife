//! Response body byte caps (`policy.egress.max_response_bytes`).

use serde_json::Value;
use types::ErrorCode;

/// Binding-frozen max response body size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResponseByteCap {
    /// `None` = unlimited.
    max_bytes: Option<u64>,
}

impl ResponseByteCap {
    #[must_use]
    pub fn unlimited() -> Self {
        Self { max_bytes: None }
    }

    #[must_use]
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes: Some(max_bytes),
        }
    }

    /// Parse `{ "egress": { "max_response_bytes": 65536 } }`.
    #[must_use]
    pub fn from_policy(policy: &Value) -> Self {
        let Some(n) = policy
            .pointer("/egress/max_response_bytes")
            .and_then(Value::as_u64)
        else {
            return Self::unlimited();
        };
        Self::new(n)
    }

    #[must_use]
    pub fn max_bytes(&self) -> Option<u64> {
        self.max_bytes
    }

    /// Reject when `len` exceeds the cap.
    ///
    /// # Errors
    /// [`ErrorCode::BudgetExhausted`] when over cap.
    pub fn permits_len(&self, len: u64) -> Result<(), ErrorCode> {
        match self.max_bytes {
            Some(max) if len > max => Err(ErrorCode::BudgetExhausted),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unlimited_when_missing() {
        let c = ResponseByteCap::from_policy(&json!({}));
        assert!(c.permits_len(u64::MAX).is_ok());
    }

    #[test]
    fn rejects_over_cap() {
        let c = ResponseByteCap::from_policy(&json!({
            "egress": { "max_response_bytes": 10 }
        }));
        assert!(c.permits_len(10).is_ok());
        assert_eq!(c.permits_len(11), Err(ErrorCode::BudgetExhausted));
    }
}
