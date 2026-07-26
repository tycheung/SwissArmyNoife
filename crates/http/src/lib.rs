//! `SwissArmyNoife` local HTTP admin surface (`sak363`) — list installed modules only.

mod routes;
mod state;

use axum::Router;

pub use routes::{
    audit_router, bindings_router, capacity_router, compute_router, connections_router,
    health_router, metrics_router, modules_router,
};
pub use state::AppState;

/// Compose the admin app router with empty in-memory state.
pub fn app() -> Router {
    app_with_state(AppState::from_env())
}

/// Compose the admin app router with shared state.
pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .merge(health_router())
        .merge(modules_router())
        .merge(capacity_router())
        .merge(compute_router())
        .merge(bindings_router())
        .merge(connections_router())
        .merge(audit_router())
        .merge(metrics_router())
        .with_state(state)
}
