//! Sandbox exec backends. `none` = host+jail; `stub` = no process spawn.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;
use types::ErrorCode;

use crate::{FilesystemJail, JailError};

/// Request to run a command inside a sandbox backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecRequest {
    /// Argv; first element is the program (PATH lookup allowed for bare names).
    pub argv: Vec<String>,
    /// Working directory (relative to jail root or absolute inside jail).
    pub cwd: PathBuf,
}

/// Captured process result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Backend / exec failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SandboxError {
    #[error("{0}")]
    Jail(#[from] JailError),
    #[error("schema.invalid: {0}")]
    SchemaInvalid(&'static str),
    #[error("sandbox.violation:{0}: {1}")]
    Violation(&'static str, String),
    #[error("sandbox.violation:spawn_failed: {0}")]
    Spawn(String),
}

impl SandboxError {
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::Jail(e) => e.to_error_code(),
            Self::SchemaInvalid(_) => ErrorCode::SchemaInvalid,
            Self::Violation(..) => ErrorCode::SandboxViolation,
            Self::Spawn(_) => ErrorCode::ProviderUnreachable,
        }
    }
}

/// Pluggable sandbox execution surface.
pub trait SandboxBackend {
    /// Run `req` and capture stdout/stderr/exit.
    ///
    /// # Errors
    /// Returns [`SandboxError`] on jail escape, bad args, or spawn failure.
    fn exec(&self, req: &ExecRequest) -> Result<ExecResult, SandboxError>;
}

pub(crate) fn validate_argv(argv: &[String]) -> Result<&str, SandboxError> {
    argv.first()
        .filter(|s| !s.is_empty())
        .map(String::as_str)
        .ok_or(SandboxError::SchemaInvalid("argv must be non-empty"))
}

/// Host process backend with cwd constrained by [`FilesystemJail`] (`backend = none`).
#[derive(Clone, Debug)]
pub struct NoneBackend {
    jail: FilesystemJail,
}

impl NoneBackend {
    #[must_use]
    pub fn new(jail: FilesystemJail) -> Self {
        Self { jail }
    }

    #[must_use]
    pub fn jail(&self) -> &FilesystemJail {
        &self.jail
    }

    /// Build from an absolute jail root.
    ///
    /// # Errors
    /// Propagates [`JailError`] when the root is invalid.
    pub fn with_root(root: impl AsRef<Path>) -> Result<Self, JailError> {
        Ok(Self::new(FilesystemJail::new(root)?))
    }
}

impl SandboxBackend for NoneBackend {
    fn exec(&self, req: &ExecRequest) -> Result<ExecResult, SandboxError> {
        let program = validate_argv(&req.argv)?;
        let cwd = self.jail.resolve(&req.cwd)?;
        let mut cmd = Command::new(program);
        if req.argv.len() > 1 {
            cmd.args(&req.argv[1..]);
        }
        cmd.current_dir(&cwd);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd
            .output()
            .map_err(|e| SandboxError::Spawn(e.to_string()))?;
        let exit_code = output.status.code().unwrap_or(-1);
        Ok(ExecResult {
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Deterministic no-spawn backend (`backend = stub`): jail-checks cwd, echoes argv.
#[derive(Clone, Debug)]
pub struct StubBackend {
    jail: FilesystemJail,
}

impl StubBackend {
    #[must_use]
    pub fn new(jail: FilesystemJail) -> Self {
        Self { jail }
    }

    #[must_use]
    pub fn jail(&self) -> &FilesystemJail {
        &self.jail
    }

    /// Build from an absolute jail root.
    ///
    /// # Errors
    /// Propagates [`JailError`] when the root is invalid.
    pub fn with_root(root: impl AsRef<Path>) -> Result<Self, JailError> {
        Ok(Self::new(FilesystemJail::new(root)?))
    }
}

impl SandboxBackend for StubBackend {
    fn exec(&self, req: &ExecRequest) -> Result<ExecResult, SandboxError> {
        let _program = validate_argv(&req.argv)?;
        let _cwd = self.jail.resolve(&req.cwd)?;
        Ok(ExecResult {
            exit_code: 0,
            stdout: format!("stub:{}", req.argv.join("\u{1f}")),
            stderr: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none_backend() -> (tempfile::TempDir, NoneBackend) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backend = NoneBackend::with_root(tmp.path()).expect("backend");
        (tmp, backend)
    }

    fn stub_backend() -> (tempfile::TempDir, StubBackend) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backend = StubBackend::with_root(tmp.path()).expect("backend");
        (tmp, backend)
    }

    #[test]
    fn host_echo_succeeds_in_jail_cwd() {
        let (_tmp, backend) = none_backend();
        let req = if cfg!(windows) {
            ExecRequest {
                argv: vec!["cmd".into(), "/C".into(), "echo hello".into()],
                cwd: PathBuf::from("."),
            }
        } else {
            ExecRequest {
                argv: vec!["echo".into(), "hello".into()],
                cwd: PathBuf::from("."),
            }
        };
        let out = backend.exec(&req).expect("exec");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.to_lowercase().contains("hello"));
    }

    #[test]
    fn cwd_escape_is_sandbox_violation() {
        let (_tmp, backend) = none_backend();
        let err = backend
            .exec(&ExecRequest {
                argv: vec!["echo".into(), "x".into()],
                cwd: PathBuf::from(".."),
            })
            .expect_err("escape");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }

    #[test]
    fn empty_argv_is_schema_invalid() {
        let (_tmp, backend) = none_backend();
        let err = backend
            .exec(&ExecRequest {
                argv: vec![],
                cwd: PathBuf::from("."),
            })
            .expect_err("empty");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }

    #[test]
    fn writes_land_inside_jail() {
        let (tmp, backend) = none_backend();
        let marker = "none-backend.txt";
        let req = if cfg!(windows) {
            ExecRequest {
                argv: vec!["cmd".into(), "/C".into(), format!("echo marker> {marker}")],
                cwd: PathBuf::from("."),
            }
        } else {
            ExecRequest {
                argv: vec!["sh".into(), "-c".into(), format!("echo marker > {marker}")],
                cwd: PathBuf::from("."),
            }
        };
        let out = backend.exec(&req).expect("exec");
        assert_eq!(out.exit_code, 0);
        let path = tmp.path().join(marker);
        let body = std::fs::read_to_string(&path).expect("read");
        assert!(body.contains("marker"));
    }

    #[test]
    fn stub_echoes_argv_without_spawn() {
        let (tmp, backend) = stub_backend();
        let out = backend
            .exec(&ExecRequest {
                argv: vec!["rm".into(), "-rf".into(), "/".into()],
                cwd: PathBuf::from("."),
            })
            .expect("stub");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout, "stub:rm\u{1f}-rf\u{1f}/");
        assert!(out.stderr.is_empty());
        // Dangerous argv must not touch the filesystem.
        assert!(!tmp.path().join("gone").exists());
    }

    #[test]
    fn stub_still_enforces_jail() {
        let (_tmp, backend) = stub_backend();
        let err = backend
            .exec(&ExecRequest {
                argv: vec!["true".into()],
                cwd: PathBuf::from(".."),
            })
            .expect_err("escape");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }
}
