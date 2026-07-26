//! `POST /v1/chat/completions` — OpenAI-shaped facade (`sak540` / `sak542` streaming).

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use control::{Offer, RateLimiter};
use serde::Deserialize;
use serde_json::{json, Value};
use types::{BindingId, InvokeReq, InvokeResp, OfferId};

use crate::openai_errors::{openai_err, openai_err_code, status_for, ErrResp};
use crate::sse::{encode_completion_stream, encode_done, encode_error};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct ChatCompletionsRequest {
    #[serde(default)]
    binding_id: Option<String>,
    /// Binding for `tools.loop` when executing `tool_calls` (`sak540-c`).
    #[serde(default)]
    tools_binding_id: Option<String>,
    messages: Vec<ChatMessageIn>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageIn {
    role: String,
    /// String content only; arrays/objects (e.g. `image_url`) are refused (`sak544-b`).
    #[serde(default)]
    content: Option<Value>,
    #[serde(default)]
    tool_calls: Vec<OpenAiToolCall>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolCall {
    id: String,
    function: OpenAiFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAiFunction {
    name: String,
    /// JSON object encoded as a string (`OpenAI` wire shape).
    arguments: String,
}

fn message_text(content: Option<&Value>) -> Result<String, ErrResp> {
    match content {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(openai_err(
            StatusCode::BAD_REQUEST,
            "schema.invalid",
            "message content must be a string (multimodal parts not supported)",
        )),
    }
}

fn claim_llm() -> OfferId {
    OfferId::new("llm.chat").expect("valid")
}

fn claim_tools_loop() -> OfferId {
    OfferId::new("tools.loop").expect("valid")
}

fn parse_uuid_binding(raw: &str) -> Result<BindingId, ErrResp> {
    uuid::Uuid::parse_str(raw)
        .map(BindingId::from_uuid)
        .map_err(|_| {
            openai_err(
                StatusCode::BAD_REQUEST,
                "binding_invalid",
                "binding_id must be a UUID",
            )
        })
}

fn parse_llm_binding_id(
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
    parse_uuid_binding(binding_raw)
}

fn parse_tools_binding_id(
    body: &ChatCompletionsRequest,
    headers: &HeaderMap,
) -> Result<BindingId, ErrResp> {
    let binding_raw = body
        .tools_binding_id
        .as_deref()
        .or_else(|| {
            headers
                .get("x-sak-tools-binding-id")
                .and_then(|v| v.to_str().ok())
        })
        .ok_or_else(|| {
            openai_err(
                StatusCode::BAD_REQUEST,
                "tools_binding_required",
                "tools_binding_id required when messages include tool_calls",
            )
        })?;
    parse_uuid_binding(binding_raw)
}

fn ensure_offer_binding(
    state: &AppState,
    binding_id: BindingId,
    expect: &str,
) -> Result<(), ErrResp> {
    let store = state.bindings.lock().expect("bindings lock");
    let record = store.get(binding_id).map_err(|code| {
        openai_err(
            status_for(code),
            code.as_str(),
            "binding missing or expired",
        )
    })?;
    if record.offer_id.as_str() != expect {
        return Err(openai_err(
            StatusCode::BAD_REQUEST,
            "wrong_offer",
            format!("binding offer is {}; expected {expect}", record.offer_id),
        ));
    }
    Ok(())
}

fn completion_ok(
    model: Option<String>,
    text: &str,
    invoke_id: impl std::fmt::Display,
    finish_reason: &str,
) -> Json<Value> {
    let model = model.unwrap_or_else(|| "sak".into());
    Json(json!({
        "id": format!("chatcmpl-{invoke_id}"),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": text },
            "finish_reason": finish_reason
        }]
    }))
}

fn sse_response(body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn record_and_count(
    state: &AppState,
    binding_id: BindingId,
    offer: OfferId,
    args: &Value,
    resp: &InvokeResp,
) {
    let mut audit = state.audit.lock().expect("audit lock");
    let invoke_id = match resp {
        InvokeResp::Ok { invoke_id, .. } => *invoke_id,
        InvokeResp::Error { invoke_id, .. } => invoke_id.unwrap_or_default(),
    };
    audit.record_invoke(invoke_id, binding_id, offer, args, resp);
    *state.invoke_count.lock().expect("invoke lock") += 1;
}

fn map_tool_calls(calls: &[OpenAiToolCall]) -> Result<Vec<Value>, ErrResp> {
    let mut out = Vec::with_capacity(calls.len());
    for c in calls {
        let args: Value = serde_json::from_str(&c.function.arguments).map_err(|e| {
            openai_err(
                StatusCode::BAD_REQUEST,
                "tool_arguments_invalid",
                format!("tool_calls {}.arguments: {e}", c.id),
            )
        })?;
        out.push(json!({
            "id": c.id,
            "tool": c.function.name,
            "args": args,
        }));
    }
    Ok(out)
}

async fn run_tools_loop(
    state: &AppState,
    headers: &HeaderMap,
    body: &ChatCompletionsRequest,
    tool_calls: &[OpenAiToolCall],
) -> Result<Json<Value>, ErrResp> {
    let binding_id = parse_tools_binding_id(body, headers)?;
    ensure_offer_binding(state, binding_id, "tools.loop")?;
    let calls = map_tool_calls(tool_calls)?;
    let invoke_args = json!({
        "step_index": 0,
        "step": { "tool_calls": calls }
    });
    let resp = state
        .tools_loop
        .invoke(InvokeReq {
            binding_id,
            args: invoke_args.clone(),
            invoke_id: None,
            offer: Some(claim_tools_loop()),
        })
        .await;
    record_and_count(state, binding_id, claim_tools_loop(), &invoke_args, &resp);
    match resp {
        InvokeResp::Ok { result, invoke_id } => {
            let text = result.to_string();
            Ok(completion_ok(
                body.model.clone(),
                &text,
                invoke_id,
                "tool_calls",
            ))
        }
        InvokeResp::Error { code, message, .. } => Err(openai_err_code(code, message)),
    }
}

async fn invoke_llm(
    state: &AppState,
    headers: &HeaderMap,
    body: &ChatCompletionsRequest,
) -> Result<(BindingId, Value, InvokeResp), ErrResp> {
    let binding_id = parse_llm_binding_id(body, headers)?;
    ensure_offer_binding(state, binding_id, "llm.chat")?;
    let messages: Vec<Value> = body
        .messages
        .iter()
        .map(|m| {
            message_text(m.content.as_ref()).map(|content| {
                json!({
                    "role": m.role,
                    "content": content
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
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
    record_and_count(state, binding_id, claim_llm(), &invoke_args, &resp);
    Ok((binding_id, invoke_args, resp))
}

async fn run_llm_chat(
    state: &AppState,
    headers: &HeaderMap,
    body: &ChatCompletionsRequest,
) -> Result<Json<Value>, ErrResp> {
    let (_binding, _args, resp) = invoke_llm(state, headers, body).await?;
    match resp {
        InvokeResp::Ok { result, invoke_id } => {
            let text = result
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            Ok(completion_ok(body.model.clone(), &text, invoke_id, "stop"))
        }
        InvokeResp::Error { code, message, .. } => Err(openai_err_code(code, message)),
    }
}

async fn run_llm_chat_stream(
    state: &AppState,
    headers: &HeaderMap,
    body: &ChatCompletionsRequest,
) -> Result<Response, ErrResp> {
    let (_binding, _args, resp) = invoke_llm(state, headers, body).await?;
    let model = body.model.clone().unwrap_or_else(|| "sak".into());
    match resp {
        InvokeResp::Ok { result, invoke_id } => {
            let text = result
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let id = format!("chatcmpl-{invoke_id}");
            Ok(sse_response(encode_completion_stream(&id, &model, &text)))
        }
        InvokeResp::Error { code, message, .. } => {
            // sak542-c: offer errors on the stream as SSE data (no secrets).
            let mut body = encode_error(code.as_str(), &message);
            body.push_str(&encode_done());
            Ok(sse_response(body))
        }
    }
}

fn check_facade_rate_limit(state: &AppState) -> Result<(), ErrResp> {
    let mut lim = state.rate_limiter.lock().expect("rate lock");
    lim.check("http-facade").map_err(|_| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": {
                    "message": RateLimiter::deny_message(),
                    "type": "rate_limit_error",
                    "code": "budget.exhausted"
                }
            })),
        )
    })
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChatCompletionsRequest>,
) -> Result<Response, ErrResp> {
    if body.messages.is_empty() {
        return Err(openai_err(
            StatusCode::BAD_REQUEST,
            "messages_required",
            "messages must be non-empty",
        ));
    }
    check_facade_rate_limit(&state)?;
    if let Some(last) = body.messages.last() {
        if !last.tool_calls.is_empty() {
            // sak543-a: tools path does not stream in v0.
            if body.stream == Some(true) {
                return Err(openai_err(
                    StatusCode::BAD_REQUEST,
                    "stream_not_supported",
                    "stream=true is not supported for tool_calls",
                ));
            }
            return Ok(run_tools_loop(&state, &headers, &body, &last.tool_calls)
                .await?
                .into_response());
        }
    }
    if body.stream == Some(true) {
        return run_llm_chat_stream(&state, &headers, &body).await;
    }
    Ok(run_llm_chat(&state, &headers, &body).await?.into_response())
}

/// OpenAI-compatible chat completions router.
pub fn chat_completions_router() -> Router<AppState> {
    Router::new().route("/v1/chat/completions", post(chat_completions))
}
