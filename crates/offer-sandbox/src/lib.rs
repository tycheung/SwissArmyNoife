//! `sandbox.*` helpers (filesystem jail + exec backends + offer).

mod backend;
mod bwrap;
mod capture;
mod docker;
mod exec_offer;
mod jail;
mod jail_offer;
mod mount_policy;
mod program_policy;
mod sanitized_env;

pub use backend::{
    unshare_net_argv, ExecRequest, ExecResult, NoneBackend, SandboxBackend, SandboxError,
    StubBackend,
};
pub use bwrap::BwrapBackend;
pub use docker::DockerBackend;
pub use exec_offer::SandboxExecOffer;
pub use jail::{FilesystemJail, JailError};
pub use jail_offer::SandboxJailOffer;
pub use mount_policy::{BindMount, MountPolicyError, WorkspaceMountPolicy};
pub use program_policy::ProgramAllowlist;
pub use sanitized_env::SanitizedEnv;
