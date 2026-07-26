//! `SwissArmyNoife` MCP library (stdio + Streamable HTTP binaries).

pub mod capacity_fit;
pub mod dispatch;
pub mod echo_offer;
pub mod health_snap;
pub mod http_auth;
pub mod live;
pub mod progress;
pub mod resources;
pub mod schema_json;
pub mod server;
pub mod session;
pub mod tool_args;
mod tool_impls;
pub mod tool_schemas;
pub mod util;
pub mod workspace_tools;

pub use server::McpServer;
pub use tool_schemas::tool_input_schemas;
