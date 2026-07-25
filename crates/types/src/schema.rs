//! JSON Schema export for core wire types.

use schemars::{schema_for, JsonSchema};
use serde_json::{json, Map, Value};

use crate::{BindingId, ErrorCode, InvokeId, InvokeReq, InvokeResp, OfferId};

/// Build a definitions document for core `types` wire structs.
#[must_use]
pub fn core_schema_document() -> Value {
    let mut definitions = Map::new();
    insert_def::<OfferId>(&mut definitions, "OfferId");
    insert_def::<BindingId>(&mut definitions, "BindingId");
    insert_def::<InvokeId>(&mut definitions, "InvokeId");
    insert_def::<ErrorCode>(&mut definitions, "ErrorCode");
    insert_def::<InvokeReq>(&mut definitions, "InvokeReq");
    insert_def::<InvokeResp>(&mut definitions, "InvokeResp");

    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "types",
        "version": env!("CARGO_PKG_VERSION"),
        "definitions": definitions,
    })
}

fn insert_def<T: JsonSchema>(definitions: &mut Map<String, Value>, name: &str) {
    let schema = schema_for!(T);
    let value = serde_json::to_value(schema).unwrap_or_else(|_| json!({}));
    definitions.insert(name.to_owned(), value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_schema_includes_invoke_and_offer() {
        let doc = core_schema_document();
        let defs = doc
            .get("definitions")
            .and_then(Value::as_object)
            .expect("definitions object");
        for key in [
            "OfferId",
            "BindingId",
            "InvokeId",
            "ErrorCode",
            "InvokeReq",
            "InvokeResp",
        ] {
            assert!(defs.contains_key(key), "missing definition {key}");
        }
        let req = defs.get("InvokeReq").expect("InvokeReq");
        let req_str = req.to_string();
        assert!(
            req_str.contains("binding_id") || req_str.contains("bindingId"),
            "InvokeReq schema should mention binding_id: {req_str}"
        );
    }
}
