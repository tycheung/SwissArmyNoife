//! MCP resources: `offer://{id}`, `binding://{id}`.

use std::time::{SystemTime, UNIX_EPOCH};

use control::{redact_json, BindingStore, CatalogRegistry};
use rmcp::{
    model::{AnnotateAble, ListResourcesResult, RawResource, ReadResourceResult, ResourceContents},
    ErrorData as McpError,
};
use serde_json::json;
use types::{BindingId, OfferId};
use uuid::Uuid;

const OFFER_PREFIX: &str = "offer://";
const BINDING_PREFIX: &str = "binding://";

/// List catalog offers and live bindings as MCP resources.
#[must_use]
pub fn list_resources(catalog: &CatalogRegistry, bindings: &BindingStore) -> ListResourcesResult {
    let mut resources = offer_resource_rows(catalog);
    resources.extend(binding_resource_rows(bindings));
    ListResourcesResult::with_all_items(resources)
}

fn offer_resource_rows(catalog: &CatalogRegistry) -> Vec<rmcp::model::Resource> {
    catalog
        .list()
        .into_iter()
        .map(|entry| {
            let uri = format!("{OFFER_PREFIX}{}", entry.id.as_str());
            let mut raw = RawResource::new(uri, entry.id.as_str());
            raw.description = Some(format!("Catalog offer {}@{}", entry.id, entry.version));
            raw.mime_type = Some("application/json".into());
            raw.no_annotation()
        })
        .collect()
}

fn binding_resource_rows(bindings: &BindingStore) -> Vec<rmcp::model::Resource> {
    bindings
        .list()
        .into_iter()
        .map(|record| {
            let uri = format!("{BINDING_PREFIX}{}", record.binding_id);
            let mut raw = RawResource::new(uri, record.binding_id.to_string());
            raw.description = Some(format!(
                "Binding for {} (principal {})",
                record.offer_id,
                record.principal.as_str()
            ));
            raw.mime_type = Some("application/json".into());
            raw.no_annotation()
        })
        .collect()
}

/// Read `offer://{id}` or `binding://{id}`.
///
/// # Errors
/// Returns MCP `invalid_params` when the URI is unknown or the target is missing.
pub fn read_resource(
    catalog: &CatalogRegistry,
    bindings: &BindingStore,
    uri: &str,
) -> Result<ReadResourceResult, McpError> {
    if let Some(raw_id) = uri.strip_prefix(OFFER_PREFIX) {
        return read_offer(catalog, uri, raw_id);
    }
    if let Some(raw_id) = uri.strip_prefix(BINDING_PREFIX) {
        return read_binding(bindings, uri, raw_id);
    }
    Err(McpError::invalid_params(
        format!("schema.invalid: unsupported resource URI {uri}"),
        None,
    ))
}

fn read_offer(
    catalog: &CatalogRegistry,
    uri: &str,
    raw_id: &str,
) -> Result<ReadResourceResult, McpError> {
    let id = OfferId::new(raw_id)
        .map_err(|code| McpError::invalid_params(format!("{code}: bad offer id in URI"), None))?;
    let entry = catalog
        .get(&id)
        .map_err(|code| McpError::invalid_params(format!("{code}: offer not in catalog"), None))?;
    Ok(text_json(
        uri,
        &json!({
            "id": entry.id.as_str(),
            "version": entry.version,
        }),
    ))
}

fn read_binding(
    bindings: &BindingStore,
    uri: &str,
    raw_id: &str,
) -> Result<ReadResourceResult, McpError> {
    let uuid = Uuid::parse_str(raw_id)
        .map_err(|_| McpError::invalid_params("schema.invalid: bad binding_id in URI", None))?;
    let id = BindingId::from_uuid(uuid);
    let record = bindings.get(id).map_err(|code| {
        McpError::invalid_params(format!("{code}: binding missing or expired"), None)
    })?;
    Ok(text_json(
        uri,
        &json!({
            "binding_id": record.binding_id.to_string(),
            "offer_id": record.offer_id.as_str(),
            "principal": record.principal.as_str(),
            "principal_kind": record.principal.kind.as_str(),
            "expires_at": expires_unix(record.expires_at),
            "policy": redact_json(&record.policy_json),
        }),
    ))
}

fn text_json(uri: &str, value: &serde_json::Value) -> ReadResourceResult {
    ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_owned(),
            mime_type: Some("application/json".into()),
            text: value.to_string(),
            meta: None,
        }],
    }
}

fn expires_unix(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use control::{BindRequest, CatalogEntry};
    use serde_json::json;

    fn catalog() -> CatalogRegistry {
        let mut c = CatalogRegistry::new();
        c.register(CatalogEntry::new("llm.chat", "0.1.0").expect("valid"));
        c
    }

    #[test]
    fn list_includes_offer_and_binding_uris() {
        let catalog = catalog();
        let mut store = BindingStore::new();
        let record = store.bind(BindRequest {
            offer_id: OfferId::new("llm.chat").expect("valid"),
            principal: control::Principal::local(),
            policy_json: json!({"api_key": "sk-secret"}),
            ttl: Duration::from_secs(60),
        });

        let listed = list_resources(&catalog, &store);
        let uris: Vec<_> = listed.resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&"offer://llm.chat"));
        let binding_uri = format!("binding://{}", record.binding_id);
        assert!(uris.contains(&binding_uri.as_str()));
    }

    #[test]
    fn read_binding_redacts_policy_secrets() {
        let catalog = catalog();
        let mut store = BindingStore::new();
        let record = store.bind(BindRequest {
            offer_id: OfferId::new("llm.chat").expect("valid"),
            principal: control::Principal::local(),
            policy_json: json!({"api_key": "sk-secret", "caps": 1}),
            ttl: Duration::from_secs(60),
        });
        let uri = format!("binding://{}", record.binding_id);
        let result = read_resource(&catalog, &store, &uri).expect("read");
        match &result.contents[0] {
            ResourceContents::TextResourceContents { text, .. } => {
                assert!(text.contains("[REDACTED]"));
                assert!(!text.contains("sk-secret"));
                assert!(text.contains("\"caps\":1"));
            }
            ResourceContents::BlobResourceContents { .. } => panic!("expected text"),
        }
    }

    #[test]
    fn read_missing_offer_is_error() {
        let err = read_resource(&catalog(), &BindingStore::new(), "offer://missing.offer")
            .expect_err("x");
        assert!(err.message.contains("offer.not_found"));
    }
}
