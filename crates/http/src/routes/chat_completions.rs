//! `POST /v1/chat/completions` — OpenAI-shaped facade over `llm.chat` (`sak540-b`).

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use control::Offer;
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, ErrorCode, InvokeReq, InvokeResp, OfferId};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct ChatCompletionsRequest {
    #[serde(default)]
    binding_id: Option<String>,
    messages: Vec<ChatMessageIn>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageIn {
    role: String,
    content: String,
}

type ErrResp = (StatusCode, Json<Value>);

fn openai_err(status: StatusCode, code: &str, message: impl Into<String>) -> ErrResp {
    (
        status,
        Json(json!({
            "error": {
                "message": message.into(),
                "type": "invalid_request_error",
                "code": code
            }
        })),
    )
}

fn status_for(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::PolicyDenied | ErrorCode::EgressDenied => StatusCode::FORBIDDEN,
        ErrorCode::BindingExpired | ErrorCode::OfferNotFound | ErrorCode::VaultMissing => {
            StatusCode::NOT_FOUND
        }
        ErrorCode::SchemaInvalid => StatusCode::BAD_REQUEST,
        ErrorCode::BudgetExhausted => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::BAD_GATEWAY,
    }
}

fn claim_llm() -> OfferId {
    OfferId::new("llm.chat").expect("valid")
}

fn parse_binding_id(
    body: &ChatCompletionsRequest,
    headers: &HeaderMap,
) -> Result<BindingId, ErrResp> {
    let binding_raw = body
        .binding_id
        .as_deref()
        .or_else(|| {
            headers
                .get("x-sak-binding-id")
                .and_then(|v| v.to_str().ok())
        })
        .ok_or_else(|| {
            openai_err(
                StatusCode::BAD_REQUEST,
                "binding_required",
                "binding_id required (body or X-Sak-Binding-Id)",
            )
        })?;
    uuid::Uuid::parse_str(binding_raw)
        .map(BindingId::from_uuid)
        .map_err(|_| {
            openai_err(
                StatusCode::BAD_REQUEST,
                "binding_invalid",
                "binding_id must be a UUID",
            )
        })
}

fn ensure_llm_chat_binding(state: &AppState, binding_id: BindingId) -> Result<(), ErrResp> {
    let store = state.bindings.lock().expect("bindings lock");
    let record = store.get(binding_id).map_err(|code| {
        openai_err(
            status_for(code),
            code.as_str(),
            "binding missing or expired",
        )
    })?;
    if record.offer_id.as_str() != "llm.chat" {
        return Err(openai_err(
            StatusCode::BAD_REQUEST,
            "wrong_offer",
            format!("binding offer is {}; expected llm.chat", record.offer_id),
        ));
    }
    Ok(())
}

fn completion_ok(
    model: Option<String>,
    text: &str,
    invoke_id: impl std::fmt::Display,
) -> Json<Value> {
    let model = model.unwrap_or_else(|| "sak".into());
    Json(json!({
        "id": format!("chatcmpl-{invoke_id}"),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": text },
            "finish_reason": "stop"
        }]
    }))
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChatCompletionsRequest>,
) -> Result<Json<Value>, ErrResp> {
    if body.stream == Some(true) {
        return Err(openai_err(
            StatusCode::BAD_REQUEST,
            "stream_not_supported",
            "stream=true is not supported in v0",
        ));
    }
    if body.messages.is_empty() {
        return Err(openai_err(
            StatusCode::BAD_REQUEST,
            "messages_required",
            "messages must be non-empty",
        ));
    }
    let binding_id = parse_binding_id(&body, &headers)?;
    ensure_llm_chat_binding(&state, binding_id)?;

    let messages: Vec<Value> = body
        .messages
        .iter()
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();
    let invoke_args = json!({ "messages": messages, "model": body.model });
    let resp = state
        .llm
        .invoke(InvokeReq {
            binding_id,
            args: invoke_args.clone(),
            invoke_id: None,
            offer: Some(claim_llm()),
        })
        .await;

    {
        let mut audit = state.audit.lock().expect("audit lock");
        let invoke_id = match &resp {
            InvokeResp::Ok { invoke_id, .. } => *invoke_id,
            InvokeResp::Error { invoke_id, .. } => invoke_id.unwrap_or_default(),
        };
        audit.record_invoke(invoke_id, binding_id, claim_llm(), &invoke_args, &resp);
    }
    *state.invoke_count.lock().expect("invoke lock") += 1;

    match resp {
        InvokeResp::Ok { result, invoke_id } => {
            let text = result
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            Ok(completion_ok(body.model.clone(), &text, invoke_id))
        }
        InvokeResp::Error { code, message, .. } => {
            Err(openai_err(status_for(code), code.as_str(), message))
        }
    }
}

/// OpenAI-compatible chat completions router.
pub fn chat_completions_router() -> Router<AppState> {
    Router::new().route("/v1/chat/completions", post(chat_completions))
}
