//! SwissArmyNoife local HTTP admin surface (`sak363`) — list installed modules only.

mod routes;
mod state;

use axum::Router;

pub use routes::{
    bindings_router, capacity_router, compute_router, health_router, metrics_router, modules_router,
};
pub use state::AppState;

/// Compose the admin app router with empty in-memory state.
#[must_use]
pub fn app() -> Router {
    app_with_state(AppState::from_env())
}

/// Compose the admin app router with shared state.
#[must_use]
pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .merge(health_router())
        .merge(modules_router())
        .merge(capacity_router())
        .merge(compute_router())
        .merge(bindings_router())
        .merge(metrics_router())
        .with_state(state)
}
