//! In-memory tool registry with JSON Schema descriptors.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use types::ErrorCode;

/// Catalogued tool: stable id + description + JSON Schema for arguments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub id: String,
    pub description: String,
    /// JSON Schema (`type: object` expected for `validate_args`).
    pub input_schema: Value,
}

impl ToolSpec {
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when `id` is empty.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Result<Self, ErrorCode> {
        let id = id.into();
        if id.is_empty() {
            return Err(ErrorCode::SchemaInvalid);
        }
        Ok(Self {
            id,
            description: description.into(),
            input_schema,
        })
    }
}

/// Process-local registry of tool specs (ordered by id for stable list).
#[derive(Clone, Debug, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolSpec>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a tool by id.
    ///
    /// # Errors
    /// Returns [`ErrorCode::SchemaInvalid`] when `spec.id` is empty.
    pub fn register(&mut self, spec: ToolSpec) -> Result<(), ErrorCode> {
        if spec.id.is_empty() {
            return Err(ErrorCode::SchemaInvalid);
        }
        self.tools.insert(spec.id.clone(), spec);
        Ok(())
    }

    /// Fetch one tool.
    ///
    /// # Errors
    /// Returns [`ErrorCode::OfferNotFound`] when the id is unknown.
    pub fn get(&self, id: &str) -> Result<&ToolSpec, ErrorCode> {
        self.tools.get(id).ok_or(ErrorCode::OfferNotFound)
    }

    /// All tools sorted by id.
    #[must_use]
    pub fn list(&self) -> Vec<&ToolSpec> {
        self.tools.values().collect()
    }

    /// Lightweight arg check: object type + `required` properties present.
    ///
    /// # Errors
    /// Returns [`ErrorCode::OfferNotFound`] / [`ErrorCode::SchemaInvalid`].
    pub fn validate_args(&self, id: &str, args: &Value) -> Result<(), ErrorCode> {
        let spec = self.get(id)?;
        let schema = &spec.input_schema;
        if schema.get("type").and_then(Value::as_str) != Some("object") {
            return Err(ErrorCode::SchemaInvalid);
        }
        let Some(obj) = args.as_object() else {
            return Err(ErrorCode::SchemaInvalid);
        };
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required {
                let Some(name) = key.as_str() else {
                    return Err(ErrorCode::SchemaInvalid);
                };
                if !obj.contains_key(name) {
                    return Err(ErrorCode::SchemaInvalid);
                }
            }
        }
        Ok(())
    }

    /// Allowlist gate then registry lookup (binding policy before schema).
    ///
    /// # Errors
    /// [`ErrorCode::PolicyDenied`] / [`ErrorCode::OfferNotFound`].
    pub fn get_allowed<'a>(
        &'a self,
        allowlist: &crate::ToolAllowlist,
        id: &str,
    ) -> Result<&'a ToolSpec, ErrorCode> {
        allowlist.permits(id)?;
        self.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn echo_spec() -> ToolSpec {
        ToolSpec::new(
            "tools.echo",
            "Echo a message",
            json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            }),
        )
        .expect("spec")
    }

    #[test]
    fn register_list_get_roundtrip() {
        let mut reg = ToolRegistry::new();
        reg.register(echo_spec()).expect("register");
        reg.register(
            ToolSpec::new(
                "tools.ping",
                "Ping",
                json!({"type": "object", "properties": {}}),
            )
            .expect("spec"),
        )
        .expect("register");

        let listed = reg.list();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "tools.echo");
        assert_eq!(listed[1].id, "tools.ping");

        let echo = reg.get("tools.echo").expect("get");
        assert_eq!(echo.description, "Echo a message");
        assert!(echo.input_schema["properties"]["message"].is_object());
    }

    #[test]
    fn validate_args_requires_fields() {
        let mut reg = ToolRegistry::new();
        reg.register(echo_spec()).expect("register");
        reg.validate_args("tools.echo", &json!({"message": "hi"}))
            .expect("ok");
        assert_eq!(
            reg.validate_args("tools.echo", &json!({})),
            Err(ErrorCode::SchemaInvalid)
        );
        assert_eq!(
            reg.validate_args("missing", &json!({})),
            Err(ErrorCode::OfferNotFound)
        );
    }

    #[test]
    fn empty_id_rejected() {
        assert_eq!(
            ToolSpec::new("", "x", json!({"type": "object"})),
            Err(ErrorCode::SchemaInvalid)
        );
    }

    #[test]
    fn replace_same_id() {
        let mut reg = ToolRegistry::new();
        reg.register(echo_spec()).expect("register");
        reg.register(
            ToolSpec::new(
                "tools.echo",
                "updated",
                json!({"type": "object", "properties": {}}),
            )
            .expect("spec"),
        )
        .expect("replace");
        assert_eq!(reg.get("tools.echo").expect("get").description, "updated");
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn get_allowed_enforces_binding_allowlist() {
        use crate::ToolAllowlist;

        let mut reg = ToolRegistry::new();
        reg.register(echo_spec()).expect("register");
        let deny = ToolAllowlist::from_policy(&json!({ "tools": { "allow": ["shell"] } }));
        assert_eq!(
            reg.get_allowed(&deny, "tools.echo").err(),
            Some(ErrorCode::PolicyDenied)
        );
        let allow = ToolAllowlist::from_policy(&json!({
            "tools": { "allow": ["tools.echo"] }
        }));
        assert_eq!(
            reg.get_allowed(&allow, "tools.echo").unwrap().id,
            "tools.echo"
        );
    }
}
