//! `tools.*` helpers (registry, allowlist, loop, fs, shell).

mod agent_loop;
mod allowlist;
mod dual_output;
mod fs;
mod loop_offer;
mod loop_types;
mod microcompact;
mod offload;
mod registry;
mod registry_offer;
mod shell;

pub use agent_loop::{AgentLoop, ToolExecutor};
pub use allowlist::ToolAllowlist;
pub use dual_output::DualOutput;
pub use fs::{fs_tool_specs, FsError, FsTools, GrepHit, ReadMode};
pub use loop_offer::ToolsLoopOffer;
pub use loop_types::{AgentStep, LoopBudget, ToolCall, ToolResult};
pub use microcompact::Microcompact;
pub use offload::{OffloadRef, ResultOffload, DEFAULT_INLINE_LIMIT};
pub use registry::{ToolRegistry, ToolSpec};
pub use registry_offer::ToolsRegistryOffer;
pub use shell::{
    shell_tool_spec, HostShellRunner, ShellError, ShellRequest, ShellResult, ShellRunner,
    ShellTools, StubShellRunner,
};
