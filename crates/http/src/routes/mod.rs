//! Route modules (`refactor:http-routes-split`).

mod bindings;
mod capacity;
mod compute;
mod connections;
mod health;
mod metrics;
mod modules;

pub use bindings::bindings_router;
pub use capacity::capacity_router;
pub use compute::compute_router;
pub use connections::connections_router;
pub use health::health_router;
pub use metrics::metrics_router;
pub use modules::modules_router;
