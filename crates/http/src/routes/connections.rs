//! Vault connection admin endpoints (`sak527-a`) — metadata only (no secret echo).

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use vault::SecretString;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct CreateConnection {
    #[serde(default)]
    connection_id: Option<String>,
    provider: String,
    #[serde(default)]
    label: String,
    /// Accepted on write only — never returned in responses.
    secret: String,
}

fn meta_json(m: &persist_sqlite::ConnectionMeta) -> Value {
    json!({
        "connection_id": m.connection_id,
        "provider": m.provider,
        "label": m.label,
    })
}

async fn list_connections(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let vault = state
        .vault
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let conn = vault
        .conn
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let listed =
        persist_sqlite::list_connections(&conn).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({
        "connections": listed.iter().map(meta_json).collect::<Vec<_>>(),
    })))
}

async fn create_connection(
    State(state): State<AppState>,
    Json(body): Json<CreateConnection>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let vault = state
        .vault
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    if body.provider.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.secret.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let connection_id = body
        .connection_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let secret = SecretString::new(body.secret);
    {
        let conn = vault
            .conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        persist_sqlite::put_connection(
            &conn,
            &vault.key,
            &connection_id,
            body.provider.trim(),
            body.label.trim(),
            &secret,
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    // Response metadata only — never echo secret.
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "connection_id": connection_id,
            "provider": body.provider.trim(),
            "label": body.label.trim(),
        })),
    ))
}

pub fn connections_router() -> Router<AppState> {
    Router::new().route(
        "/v1/sak/connections",
        get(list_connections).post(create_connection),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_body_deserializes_without_id() {
        let v: CreateConnection = serde_json::from_value(json!({
            "provider": "openai",
            "label": "prod",
            "secret": "sk-test"
        }))
        .expect("de");
        assert!(v.connection_id.is_none());
        assert_eq!(v.provider, "openai");
    }
}
