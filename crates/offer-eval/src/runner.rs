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
        "regex" => eval_regex(c),
        "numeric_tolerance" => eval_numeric_tolerance(c),
        "contains_all" => eval_contains_set(c, true),
        "contains_any" => eval_contains_set(c, false),
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

fn eval_regex(c: &CheckSpec) -> Result<(bool, Value), (ErrorCode, String)> {
    let Some(hay) = c.actual.as_str() else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: regex requires string actual", c.id),
        ));
    };
    let Some(pat) = c.expected.as_str() else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: regex requires string expected", c.id),
        ));
    };
    let re = regex::Regex::new(pat).map_err(|e| {
        (
            ErrorCode::SchemaInvalid,
            format!("check {}: invalid regex: {e}", c.id),
        )
    })?;
    let ok = re.is_match(hay);
    let message = if ok {
        Value::Null
    } else {
        json!(format!("regex failed: {hay:?} !~ {pat:?}"))
    };
    Ok((ok, message))
}

fn as_f64(v: &Value, id: &str, field: &str) -> Result<f64, (ErrorCode, String)> {
    v.as_f64().ok_or_else(|| {
        (
            ErrorCode::SchemaInvalid,
            format!("check {id}: {field} must be numeric"),
        )
    })
}

fn eval_numeric_tolerance(c: &CheckSpec) -> Result<(bool, Value), (ErrorCode, String)> {
    let actual = as_f64(&c.actual, &c.id, "actual")?;
    let Some(expected_val) = c.expected.get("value") else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: numeric_tolerance expected.value required", c.id),
        ));
    };
    let expected = as_f64(expected_val, &c.id, "expected.value")?;
    let abs = c
        .expected
        .get("abs")
        .map(|v| as_f64(v, &c.id, "abs"))
        .transpose()?;
    let rel = c
        .expected
        .get("rel")
        .map(|v| as_f64(v, &c.id, "rel"))
        .transpose()?;
    if abs.is_none() && rel.is_none() {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: numeric_tolerance needs abs or rel", c.id),
        ));
    }
    let delta = (actual - expected).abs();
    let ok = abs.is_some_and(|a| delta <= a) || rel.is_some_and(|r| delta <= r * expected.abs());
    let message = if ok {
        Value::Null
    } else {
        json!(format!(
            "numeric_tolerance failed: actual={actual} expected={expected} delta={delta}"
        ))
    };
    Ok((ok, message))
}

fn eval_contains_set(c: &CheckSpec, all: bool) -> Result<(bool, Value), (ErrorCode, String)> {
    let Some(actual) = c.actual.as_array() else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: contains_all/any requires array actual", c.id),
        ));
    };
    let Some(expected) = c.expected.as_array() else {
        return Err((
            ErrorCode::SchemaInvalid,
            format!("check {}: contains_all/any requires array expected", c.id),
        ));
    };
    let ok = if all {
        expected.iter().all(|e| actual.contains(e))
    } else {
        expected.iter().any(|e| actual.contains(e))
    };
    let message = if ok {
        Value::Null
    } else {
        let kind = if all { "contains_all" } else { "contains_any" };
        json!(format!("{kind} failed"))
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

    #[test]
    fn regex_pass_fail_and_invalid() {
        let pass = run_checks(
            &json!({
                "checks": [{
                    "id": "re",
                    "assert": "regex",
                    "actual": "abc123",
                    "expected": "^[a-z]+[0-9]+$"
                }]
            }),
            None,
        )
        .expect("pass");
        assert_eq!(pass["passed"], true);
        let fail = run_checks(
            &json!({
                "checks": [{
                    "id": "re",
                    "assert": "regex",
                    "actual": "nope",
                    "expected": "^[0-9]+$"
                }]
            }),
            None,
        )
        .expect("fail");
        assert_eq!(fail["passed"], false);
        let err = run_checks(
            &json!({
                "checks": [{
                    "id": "re",
                    "assert": "regex",
                    "actual": "x",
                    "expected": "("
                }]
            }),
            None,
        )
        .expect_err("invalid");
        assert_eq!(err.0, ErrorCode::SchemaInvalid);
        assert!(err.1.contains("invalid regex"));
    }

    #[test]
    fn numeric_tolerance_abs_and_rel() {
        let pass = run_checks(
            &json!({
                "checks": [{
                    "id": "n",
                    "assert": "numeric_tolerance",
                    "actual": 1.005,
                    "expected": { "value": 1.0, "abs": 0.01 }
                }]
            }),
            None,
        )
        .expect("pass");
        assert_eq!(pass["passed"], true);
        let fail = run_checks(
            &json!({
                "checks": [{
                    "id": "n",
                    "assert": "numeric_tolerance",
                    "actual": 2.0,
                    "expected": { "value": 1.0, "rel": 0.01 }
                }]
            }),
            None,
        )
        .expect("fail");
        assert_eq!(fail["passed"], false);
    }

    #[test]
    fn contains_all_and_any() {
        let pass_all = run_checks(
            &json!({
                "checks": [{
                    "id": "a",
                    "assert": "contains_all",
                    "actual": ["x", "y", "z"],
                    "expected": ["x", "z"]
                }]
            }),
            None,
        )
        .expect("all");
        assert_eq!(pass_all["passed"], true);
        let pass_any = run_checks(
            &json!({
                "checks": [{
                    "id": "b",
                    "assert": "contains_any",
                    "actual": ["x"],
                    "expected": ["q", "x"]
                }]
            }),
            None,
        )
        .expect("any");
        assert_eq!(pass_any["passed"], true);
        let fail_all = run_checks(
            &json!({
                "checks": [{
                    "id": "c",
                    "assert": "contains_all",
                    "actual": ["x"],
                    "expected": ["x", "y"]
                }]
            }),
            None,
        )
        .expect("fail all");
        assert_eq!(fail_all["passed"], false);
    }
}
