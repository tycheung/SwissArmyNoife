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
///
/// When `allowed_asserts` is `Some`, unknown assert kinds return `policy.denied`.
pub(crate) fn run_checks(
    args: &Value,
    allowed_asserts: Option<&[String]>,
) -> Result<Value, (ErrorCode, String)> {
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
        if let Some(allow) = allowed_asserts {
            if !allow.iter().any(|a| a == &c.assert) {
                return Err((
                    ErrorCode::PolicyDenied,
                    format!("assert {:?} not allowed by binding policy", c.assert),
                ));
            }
        }
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
            let ok = c.actual == c.expected;
            let message = if ok {
                Value::Null
            } else {
                json!(format!(
                    "eq failed: actual={} expected={}",
                    c.actual, c.expected
                ))
            };
            Ok((ok, message))
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
            let ok = hay.contains(needle);
            let message = if ok {
                Value::Null
            } else {
                json!(format!("contains failed: {hay:?} missing {needle:?}"))
            };
            Ok((ok, message))
        }
        "json_path" => eval_json_path(c),
        other => Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: unknown assert {other:?}", c.id),
        )),
    }
}

fn eval_json_path(c: &CheckSpec) -> Result<(bool, Value), (ErrorCode, String)> {
    let Some(ptr) = c.expected.get("pointer").and_then(Value::as_str) else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!(
                "check {}: json_path expected.pointer must be a string",
                c.id
            ),
        ));
    };
    let Some(equals) = c.expected.get("equals") else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: json_path expected.equals required", c.id),
        ));
    };
    let found = c.actual.pointer(ptr);
    let ok = found == Some(equals);
    let message = if ok {
        Value::Null
    } else {
        json!(format!(
            "json_path {ptr} actual={found:?} expected={equals}"
        ))
    };
    Ok((ok, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fixture_pass_all_eq() {
        let out = run_checks(
            &json!({
                "checks": [
                    { "id": "a", "assert": "eq", "actual": 1, "expected": 1 },
                    { "id": "b", "assert": "contains", "actual": "hello", "expected": "ell" }
                ]
            }),
            None,
        )
        .expect("run");
        assert_eq!(out["passed"], true);
        assert_eq!(out["results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn fixture_fail_on_mismatch() {
        let out = run_checks(
            &json!({
                "checks": [
                    { "id": "a", "assert": "eq", "actual": 1, "expected": 2 }
                ]
            }),
            None,
        )
        .expect("run");
        assert_eq!(out["passed"], false);
        assert_eq!(out["results"][0]["passed"], false);
    }

    #[test]
    fn empty_checks_rejected() {
        let err = run_checks(&json!({ "checks": [] }), None).expect_err("empty");
        assert_eq!(err.0, ErrorCode::SchemaInvalid);
    }

    #[test]
    fn deny_disallowed_assert() {
        let allow = vec!["eq".into()];
        let err = run_checks(
            &json!({
                "checks": [
                    { "id": "c", "assert": "contains", "actual": "a", "expected": "a" }
                ]
            }),
            Some(&allow),
        )
        .expect_err("deny");
        assert_eq!(err.0, ErrorCode::PolicyDenied);
    }

    #[test]
    fn json_path_pass_and_fail() {
        let doc = json!({ "user": { "id": 7 } });
        let pass = run_checks(
            &json!({
                "checks": [{
                    "id": "jp",
                    "assert": "json_path",
                    "actual": doc,
                    "expected": { "pointer": "/user/id", "equals": 7 }
                }]
            }),
            None,
        )
        .expect("pass");
        assert_eq!(pass["passed"], true);
        let fail = run_checks(
            &json!({
                "checks": [{
                    "id": "jp",
                    "assert": "json_path",
                    "actual": { "user": { "id": 7 } },
                    "expected": { "pointer": "/user/id", "equals": 8 }
                }]
            }),
            None,
        )
        .expect("fail");
        assert_eq!(fail["passed"], false);
    }

    #[test]
    fn json_path_allowlist() {
        let allow = vec!["eq".into()];
        let err = run_checks(
            &json!({
                "checks": [{
                    "id": "jp",
                    "assert": "json_path",
                    "actual": { "a": 1 },
                    "expected": { "pointer": "/a", "equals": 1 }
                }]
            }),
            Some(&allow),
        )
        .expect_err("deny");
        assert_eq!(err.0, ErrorCode::PolicyDenied);
    }
}
