//! Generic equality / contains checks for `eval.run`.

use serde::Deserialize;
use serde_json::{json, Value};
use types::ErrorCode;

#[derive(Debug, Deserialize)]
pub(crate) struct RunArgs {
    #[serde(default = "default_op")]
    pub op: String,
    #[serde(default)]
    pub checks: Vec<CheckSpec>,
}

fn default_op() -> String {
    "run".into()
}

#[derive(Debug, Deserialize)]
pub(crate) struct CheckSpec {
    pub id: String,
    #[serde(default = "default_assert")]
    pub assert: String,
    pub actual: Value,
    pub expected: Value,
}

fn default_assert() -> String {
    "eq".into()
}

/// Run checks; overall `passed` is true iff every check passes.
pub(crate) fn run_checks(args: &Value) -> Result<Value, (ErrorCode, String)> {
    let parsed: RunArgs = serde_json::from_value(args.clone())
        .map_err(|e| (ErrorCode::SchemaInvalid, format!("eval.run args: {e}")))?;
    if parsed.op != "run" {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("unknown op {:?}; expected \"run\"", parsed.op),
        ));
    }
    if parsed.checks.is_empty() {
        return Err((
            ErrorCode::SchemaInvalid,
            "eval.run requires non-empty checks".into(),
        ));
    }
    let mut results = Vec::with_capacity(parsed.checks.len());
    let mut all_ok = true;
    for c in &parsed.checks {
        let (ok, message) = eval_one(c)?;
        if !ok {
            all_ok = false;
        }
        results.push(json!({
            "id": c.id,
            "passed": ok,
            "message": message,
        }));
    }
    Ok(json!({
        "passed": all_ok,
        "results": results,
    }))
}

fn eval_one(c: &CheckSpec) -> Result<(bool, Value), (ErrorCode, String)> {
    match c.assert.as_str() {
        "eq" => {
            let passed = c.actual == c.expected;
            let message = if passed {
                Value::Null
            } else {
                json!(format!(
                    "eq failed: actual={:?} expected={:?}",
                    c.actual, c.expected
                ))
            };
            Ok((passed, message))
        }
        "contains" => {
            let Some(hay) = c.actual.as_str() else {
                return Err((
                    ErrorCode::SchemaInvalid,
                    format!("check {}: contains requires string actual", c.id),
                ));
            };
            let Some(needle) = c.expected.as_str() else {
                return Err((
                    ErrorCode::SchemaInvalid,
                    format!("check {}: contains requires string expected", c.id),
                ));
            };
            let passed = hay.contains(needle);
            let message = if passed {
                Value::Null
            } else {
                json!(format!("contains failed: {hay:?} missing {needle:?}"))
            };
            Ok((passed, message))
        }
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: unknown assert {other:?}", c.id),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fixture_pass_all_eq() {
        let out = run_checks(&json!({
            "checks": [
                { "id": "a", "assert": "eq", "actual": 1, "expected": 1 },
                { "id": "b", "assert": "contains", "actual": "hello", "expected": "ell" }
            ]
        }))
        .expect("run");
        assert_eq!(out["passed"], true);
        assert_eq!(out["results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn fixture_fail_on_mismatch() {
        let out = run_checks(&json!({
            "checks": [
                { "id": "a", "assert": "eq", "actual": 1, "expected": 2 }
            ]
        }))
        .expect("run");
        assert_eq!(out["passed"], false);
        assert_eq!(out["results"][0]["passed"], false);
    }

    #[test]
    fn empty_checks_rejected() {
        let err = run_checks(&json!({ "checks": [] })).expect_err("empty");
        assert_eq!(err.0, ErrorCode::SchemaInvalid);
    }
}
