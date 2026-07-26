//! MCP progress / logging notification helpers (`sak109`).

use rmcp::model::{
    LoggingLevel, LoggingMessageNotificationParam, ProgressNotificationParam, ProgressToken,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, Peer, RoleServer};

/// Emit `notifications/progress` when the client supplied a progress token.
///
/// Silently no-ops when no token is present (most Cursor tool calls).
pub async fn notify_progress(
    ctx: &RequestContext<RoleServer>,
    progress: f64,
    total: Option<f64>,
    message: impl Into<String>,
) {
    let Some(token) = ctx.meta.get_progress_token() else {
        return;
    };
    let param = ProgressNotificationParam {
        progress_token: token.clone(),
        progress,
        total,
        message: Some(message.into()),
    };
    let _ = ctx.peer.notify_progress(param).await;
}

/// Emit `notifications/message` (best-effort; ignore transport errors).
pub async fn notify_log(peer: &Peer<RoleServer>, level: LoggingLevel, data: serde_json::Value) {
    let _ = peer
        .notify_logging_message(LoggingMessageNotificationParam {
            level,
            data,
            logger: Some("swissarmynoife".into()),
        })
        .await;
}

/// Build a progress param (unit-testable without a live peer).
#[must_use]
pub fn progress_param(
    token: ProgressToken,
    progress: f64,
    total: Option<f64>,
    message: impl Into<String>,
) -> ProgressNotificationParam {
    ProgressNotificationParam {
        progress_token: token,
        progress,
        total,
        message: Some(message.into()),
    }
}

/// Map a missing-token case to a soft MCP error (rarely used).
#[must_use]
pub fn missing_token_hint() -> McpError {
    McpError::invalid_params(
        "no progress token in request meta (client did not request progress)",
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::NumberOrString;

    #[test]
    #[allow(clippy::float_cmp)]
    fn progress_param_shapes() {
        let token = ProgressToken(NumberOrString::Number(1.into()));
        let p = progress_param(token, 0.5, Some(1.0), "halfway");
        assert_eq!(p.progress, 0.5);
        assert_eq!(p.total, Some(1.0));
        assert_eq!(p.message.as_deref(), Some("halfway"));
    }
}
