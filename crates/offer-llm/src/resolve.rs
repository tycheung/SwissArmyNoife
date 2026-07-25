//! LLM binding resolution (ADR 006). Metadata-only; no secrets.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use types::ErrorCode;

/// Why a resolution was chosen (safe for audit / telemetry).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingSource {
    ConnectionId,
    ProviderDefault,
    LocalOllama,
}

/// Caller hint (from invoke args or binding policy).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveHint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Catalog row visible to resolve (no ciphertext / secrets).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionRef {
    pub connection_id: String,
    pub provider: String,
    pub label: String,
}

/// Successful resolution (never carries secrets).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLlm {
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    pub binding_source: BindingSource,
}

/// Resolve failures mapped to broker [`ErrorCode`]s.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResolveError {
    #[error("vault.missing: {0}")]
    VaultMissing(String),
    #[error("schema.invalid: {0}")]
    SchemaInvalid(String),
}

impl ResolveError {
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::VaultMissing(_) => ErrorCode::VaultMissing,
            Self::SchemaInvalid(_) => ErrorCode::SchemaInvalid,
        }
    }
}

/// Resolve provider/model/connection per ADR 006 precedence.
///
/// # Errors
/// Returns [`ResolveError`] when the hint is empty, a connection is missing, or a remote
/// provider has no vault row.
pub fn resolve(hint: &ResolveHint, catalog: &[ConnectionRef]) -> Result<ResolvedLlm, ResolveError> {
    if let Some(connection_id) = hint.connection_id.as_deref() {
        return resolve_connection_id(connection_id, hint.model.as_deref(), catalog);
    }
    if let Some(provider) = hint.provider.as_deref() {
        return resolve_provider(provider, hint.model.as_deref(), catalog);
    }
    if let Some(model) = hint.model.as_deref() {
        return Ok(ResolvedLlm {
            provider: "ollama".into(),
            model: model.to_owned(),
            connection_id: None,
            binding_source: BindingSource::LocalOllama,
        });
    }
    Err(ResolveError::SchemaInvalid(
        "resolve requires connection_id, provider, and/or model".into(),
    ))
}

fn resolve_connection_id(
    connection_id: &str,
    model: Option<&str>,
    catalog: &[ConnectionRef],
) -> Result<ResolvedLlm, ResolveError> {
    let row = catalog
        .iter()
        .find(|c| c.connection_id == connection_id)
        .ok_or_else(|| ResolveError::VaultMissing(connection_id.to_owned()))?;
    let model = pick_model(model, &row.provider)?;
    Ok(ResolvedLlm {
        provider: row.provider.clone(),
        model,
        connection_id: Some(row.connection_id.clone()),
        binding_source: BindingSource::ConnectionId,
    })
}

fn resolve_provider(
    provider: &str,
    model: Option<&str>,
    catalog: &[ConnectionRef],
) -> Result<ResolvedLlm, ResolveError> {
    let model = pick_model(model, provider)?;
    if let Some(row) = pick_provider_connection(provider, catalog) {
        return Ok(ResolvedLlm {
            provider: row.provider.clone(),
            model,
            connection_id: Some(row.connection_id.clone()),
            binding_source: BindingSource::ProviderDefault,
        });
    }
    if provider == "ollama" {
        return Ok(ResolvedLlm {
            provider: "ollama".into(),
            model,
            connection_id: None,
            binding_source: BindingSource::LocalOllama,
        });
    }
    Err(ResolveError::VaultMissing(format!(
        "no vault connection for provider {provider}"
    )))
}

fn pick_provider_connection<'a>(
    provider: &str,
    catalog: &'a [ConnectionRef],
) -> Option<&'a ConnectionRef> {
    let mut matches: Vec<&ConnectionRef> =
        catalog.iter().filter(|c| c.provider == provider).collect();
    if matches.is_empty() {
        return None;
    }
    if let Some(default) = matches.iter().find(|c| c.label == "default") {
        return Some(*default);
    }
    matches.sort_by_key(|c| c.connection_id.as_str());
    matches.first().copied()
}

fn pick_model(model: Option<&str>, provider: &str) -> Result<String, ResolveError> {
    if let Some(m) = model {
        if m.is_empty() {
            return Err(ResolveError::SchemaInvalid(
                "model must be non-empty".into(),
            ));
        }
        return Ok(m.to_owned());
    }
    match provider {
        "ollama" => Ok("llama3.2".into()),
        "openai" => Ok("gpt-4o-mini".into()),
        "anthropic" => Ok("claude-3-5-haiku-latest".into()),
        other => Err(ResolveError::SchemaInvalid(format!(
            "model required for provider {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Vec<ConnectionRef> {
        vec![
            ConnectionRef {
                connection_id: "conn-openai-default".into(),
                provider: "openai".into(),
                label: "default".into(),
            },
            ConnectionRef {
                connection_id: "conn-openai-other".into(),
                provider: "openai".into(),
                label: "staging".into(),
            },
            ConnectionRef {
                connection_id: "conn-anthropic".into(),
                provider: "anthropic".into(),
                label: "work".into(),
            },
        ]
    }

    #[test]
    fn connection_id_wins() {
        let hint = ResolveHint {
            connection_id: Some("conn-anthropic".into()),
            provider: Some("openai".into()),
            model: Some("claude-custom".into()),
        };
        let got = resolve(&hint, &catalog()).expect("ok");
        assert_eq!(got.provider, "anthropic");
        assert_eq!(got.model, "claude-custom");
        assert_eq!(got.connection_id.as_deref(), Some("conn-anthropic"));
        assert_eq!(got.binding_source, BindingSource::ConnectionId);
    }

    #[test]
    fn missing_connection_is_vault_missing() {
        let hint = ResolveHint {
            connection_id: Some("missing".into()),
            ..ResolveHint::default()
        };
        let err = resolve(&hint, &catalog()).expect_err("missing");
        assert_eq!(err.to_error_code(), ErrorCode::VaultMissing);
    }

    #[test]
    fn provider_prefers_default_label() {
        let hint = ResolveHint {
            provider: Some("openai".into()),
            model: None,
            connection_id: None,
        };
        let got = resolve(&hint, &catalog()).expect("ok");
        assert_eq!(got.connection_id.as_deref(), Some("conn-openai-default"));
        assert_eq!(got.model, "gpt-4o-mini");
        assert_eq!(got.binding_source, BindingSource::ProviderDefault);
    }

    #[test]
    fn ollama_without_vault_is_local() {
        let hint = ResolveHint {
            provider: Some("ollama".into()),
            model: None,
            connection_id: None,
        };
        let got = resolve(&hint, &[]).expect("ok");
        assert_eq!(got.provider, "ollama");
        assert_eq!(got.model, "llama3.2");
        assert!(got.connection_id.is_none());
        assert_eq!(got.binding_source, BindingSource::LocalOllama);
    }

    #[test]
    fn remote_provider_without_connection_fails() {
        let hint = ResolveHint {
            provider: Some("openai".into()),
            ..ResolveHint::default()
        };
        let err = resolve(&hint, &[]).expect_err("need vault");
        assert_eq!(err.to_error_code(), ErrorCode::VaultMissing);
    }

    #[test]
    fn model_only_defaults_to_ollama() {
        let hint = ResolveHint {
            model: Some("mistral".into()),
            ..ResolveHint::default()
        };
        let got = resolve(&hint, &catalog()).expect("ok");
        assert_eq!(got.provider, "ollama");
        assert_eq!(got.model, "mistral");
        assert_eq!(got.binding_source, BindingSource::LocalOllama);
    }

    #[test]
    fn empty_hint_is_schema_invalid() {
        let err = resolve(&ResolveHint::default(), &catalog()).expect_err("empty");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }

    #[test]
    fn resolved_json_has_no_secret_shaped_fields() {
        let got = resolve(
            &ResolveHint {
                provider: Some("openai".into()),
                ..ResolveHint::default()
            },
            &catalog(),
        )
        .expect("ok");
        let v = serde_json::to_value(&got).expect("ser");
        let obj = v.as_object().expect("obj");
        assert!(!obj.contains_key("api_key"));
        assert!(!obj.contains_key("secret"));
        assert!(obj.contains_key("binding_source"));
    }
}
