//! Host-authoritative merge hooks (`sak295-a`).

use serde_json::Value;
use types::ErrorCode;

/// Merge worker result into host-visible work outcome.
pub trait MergeHook: Send + Sync {
    /// # Errors
    /// Implementation-defined schema errors.
    fn merge(&self, original_payload: &Value, worker_result: &Value) -> Result<Value, ErrorCode>;
}

/// Default: worker result wins (host still owns the store).
#[derive(Clone, Debug, Default)]
pub struct IdentityMerge;

impl MergeHook for IdentityMerge {
    fn merge(&self, _original_payload: &Value, worker_result: &Value) -> Result<Value, ErrorCode> {
        Ok(worker_result.clone())
    }
}

/// Prefer worker fields but keep host `kind` / `meta` keys from original when present.
#[derive(Clone, Debug, Default)]
pub struct PreferWorkerMerge;

impl MergeHook for PreferWorkerMerge {
    fn merge(&self, original_payload: &Value, worker_result: &Value) -> Result<Value, ErrorCode> {
        let mut out = original_payload.clone();
        match (&mut out, worker_result) {
            (Value::Object(base), Value::Object(extra)) => {
                for (k, v) in extra {
                    base.insert(k.clone(), v.clone());
                }
                Ok(out)
            }
            _ => Ok(worker_result.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identity_returns_worker() {
        let m = IdentityMerge;
        let r = m.merge(&json!({"a": 1}), &json!({"b": 2})).unwrap();
        assert_eq!(r, json!({"b": 2}));
    }

    #[test]
    fn prefer_worker_overlays() {
        let m = PreferWorkerMerge;
        let r = m
            .merge(&json!({"a": 1, "meta": true}), &json!({"a": 9, "b": 2}))
            .unwrap();
        assert_eq!(r["a"], 9);
        assert_eq!(r["b"], 2);
        assert_eq!(r["meta"], true);
    }
}
