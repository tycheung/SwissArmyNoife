//! OpenAI-ish SSE chunk encoder for chat streaming (`sak542-a`).

use serde_json::{json, Value};

/// Encode one assistant text delta as an `OpenAI` `chat.completion.chunk` SSE data line.
#[must_use]
pub fn encode_text_delta(id: &str, model: &str, index: u32, text: &str) -> String {
    let chunk = json!({
        "id": id,
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": index,
            "delta": { "content": text },
            "finish_reason": Value::Null
        }]
    });
    format!("data: {chunk}\n\n")
}

/// Encode the terminal `[DONE]` SSE sentinel.
#[must_use]
pub fn encode_done() -> String {
    "data: [DONE]\n\n".into()
}

/// Encode a stream error as an SSE data object (no secrets).
#[must_use]
pub fn encode_error(code: &str, message: &str) -> String {
    let chunk = json!({
        "error": {
            "message": message,
            "type": "invalid_request_error",
            "code": code
        }
    });
    format!("data: {chunk}\n\n")
}

/// Split assistant text into SSE frames ending with `[DONE]`.
#[must_use]
pub fn encode_completion_stream(id: &str, model: &str, text: &str) -> String {
    let mut out = String::new();
    if text.is_empty() {
        out.push_str(&encode_text_delta(id, model, 0, ""));
    } else {
        // Single delta for v0 (echo); multi-chunk polish can split later.
        out.push_str(&encode_text_delta(id, model, 0, text));
    }
    out.push_str(&encode_done());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_text_delta_and_done() {
        let body = encode_completion_stream("chatcmpl-1", "echo", "hi");
        assert!(body.contains("data: {"));
        assert!(body.contains("\"content\":\"hi\""));
        assert!(body.contains("chat.completion.chunk"));
        assert!(body.ends_with("data: [DONE]\n\n"));
    }

    #[test]
    fn encodes_error_without_done() {
        let line = encode_error("binding_invalid", "bad id");
        assert!(line.contains("\"code\":\"binding_invalid\""));
        assert!(!line.contains("[DONE]"));
    }
}
