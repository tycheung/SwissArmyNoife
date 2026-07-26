//! OpenAI-shaped error mapping for the chat facade (`sak544-a`).

use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use types::ErrorCode;

/// HTTP + JSON error body used by the chat facade.
pub type ErrResp = (StatusCode, Json<Value>);

/// Map broker [`ErrorCode`] to an HTTP status.
#[must_use]
pub fn status_for(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::PolicyDenied | ErrorCode::EgressDenied => StatusCode::FORBIDDEN,
        ErrorCode::BindingExpired | ErrorCode::OfferNotFound | ErrorCode::VaultMissing => {
            StatusCode::NOT_FOUND
        }
        ErrorCode::SchemaInvalid | ErrorCode::ModuleIncompatible | ErrorCode::SandboxViolation => {
            StatusCode::BAD_REQUEST
        }
        ErrorCode::BudgetExhausted => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::ProviderUnreachable => StatusCode::BAD_GATEWAY,
    }
}

/// Map broker [`ErrorCode`] to an OpenAI-ish `error.type`.
#[must_use]
pub fn openai_type_for(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::PolicyDenied | ErrorCode::EgressDenied => "permission_error",
        ErrorCode::BudgetExhausted => "rate_limit_error",
        ErrorCode::SchemaInvalid
        | ErrorCode::ModuleIncompatible
        | ErrorCode::SandboxViolation
        | ErrorCode::BindingExpired
        | ErrorCode::OfferNotFound
        | ErrorCode::VaultMissing => "invalid_request_error",
        ErrorCode::ProviderUnreachable => "server_error",
    }
}

/// Facade-local validation error (`binding_required`, …).
pub fn openai_err(status: StatusCode, code: &str, message: impl Into<String>) -> ErrResp {
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

/// Broker [`ErrorCode`] as an OpenAI-ish error body.
pub fn openai_err_code(code: ErrorCode, message: impl Into<String>) -> ErrResp {
    (
        status_for(code),
        Json(json!({
            "error": {
                "message": message.into(),
                "type": openai_type_for(code),
                "code": code.as_str()
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_policy_schema_budget() {
        assert_eq!(status_for(ErrorCode::PolicyDenied), StatusCode::FORBIDDEN);
        assert_eq!(openai_type_for(ErrorCode::PolicyDenied), "permission_error");

        assert_eq!(status_for(ErrorCode::SchemaInvalid), StatusCode::BAD_REQUEST);
        assert_eq!(
            openai_type_for(ErrorCode::SchemaInvalid),
            "invalid_request_error"
        );

        assert_eq!(
            status_for(ErrorCode::BudgetExhausted),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            openai_type_for(ErrorCode::BudgetExhausted),
            "rate_limit_error"
        );
    }

    #[test]
    fn openai_err_code_body() {
        let (status, Json(body)) = openai_err_code(ErrorCode::EgressDenied, "no");
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"]["type"], "permission_error");
        assert_eq!(body["error"]["code"], "egress.denied");
    }
}
