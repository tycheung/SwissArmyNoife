//! `SwissArmyNoife` local HTTP admin surface (`sak363`) — list installed modules only.

mod auth;
mod routes;
mod state;

use std::sync::Arc;

use axum::middleware::from_fn;
use axum::Router;

pub use auth::{
    auth_middleware, bearer_authorized, token_from_env, HTTP_ALLOW_INSECURE_ENV, HTTP_TOKEN_ENV,
};
pub use routes::{
    audit_router, bindings_router, capacity_router, chat_completions_router, compute_router,
    connections_router, health_router, metrics_router, modules_router,
};
pub use state::AppState;

/// Compose the admin app router with empty in-memory state.
pub fn app() -> Router {
    app_with_state(AppState::from_env())
}

/// Compose the admin app router with shared state.
pub fn app_with_state(state: AppState) -> Router {
    let expected = state.http_token.clone();
    let api_keys = Arc::clone(&state.api_keys);
    Router::new()
        .merge(health_router())
        .merge(modules_router())
        .merge(capacity_router())
        .merge(compute_router())
        .merge(bindings_router())
        .merge(connections_router())
        .merge(audit_router())
        .merge(metrics_router())
        .merge(chat_completions_router())
        .layer(from_fn(move |req, next| {
            let expected = expected.clone();
            let api_keys = Arc::clone(&api_keys);
            async move { auth_middleware(expected, api_keys, req, next).await }
        }))
        .with_state(state)
}
