//! `tools.shell` — exec via a pluggable [`ShellRunner`] (sandbox backends adapt outside).
//!
//! # Windows subprocess notes (sak204)
//!
//! - Prefer argv arrays over `cmd /C` string concatenation when the program is known.
//! - Tests use `cmd /C echo …` because `echo` is a shell builtin on Windows.
//! - [`HostShellRunner`] sets `CREATE_NO_WINDOW` so MCP/host-jail exec does not flash a console.
//! - Working directory must resolve inside the jail; absolute paths outside are violations.
//! - `COMSPEC` is not consulted; callers pass `cmd` explicitly when a shell is required.

use std::path::{Component, Path, PathBuf};
use std::process::Command;

use thiserror::Error;
use types::ErrorCode;

use crate::registry::ToolSpec;

/// Shell tool / runner errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShellError {
    #[error("sandbox.violation: path escapes jail root")]
    Escape,
    #[error("schema.invalid: {0}")]
    SchemaInvalid(&'static str),
    #[error("provider.unreachable: {0}")]
    Spawn(String),
}

impl ShellError {
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::Escape => ErrorCode::SandboxViolation,
            Self::SchemaInvalid(_) => ErrorCode::SchemaInvalid,
            Self::Spawn(_) => ErrorCode::ProviderUnreachable,
        }
    }
}

/// Request forwarded to a [`ShellRunner`] (cwd relative to the runner's jail).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellRequest {
    pub argv: Vec<String>,
    pub cwd: String,
}

/// Captured command result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Pluggable exec surface — sandbox `NoneBackend` / `StubBackend` adapt to this.
pub trait ShellRunner {
    /// # Errors
    /// Runner-specific failure (jail, spawn, schema).
    fn run(&self, req: &ShellRequest) -> Result<ShellResult, ShellError>;
}

/// `tools.shell` facade over a [`ShellRunner`].
#[derive(Clone, Debug)]
pub struct ShellTools<R> {
    runner: R,
}

impl<R> ShellTools<R> {
    #[must_use]
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    #[must_use]
    pub fn runner(&self) -> &R {
        &self.runner
    }
}

impl<R: ShellRunner> ShellTools<R> {
    /// Run `argv` with working directory `cwd` (default `.`).
    ///
    /// # Errors
    /// Empty argv or runner failure.
    pub fn exec(
        &self,
        argv: Vec<String>,
        cwd: impl Into<String>,
    ) -> Result<ShellResult, ShellError> {
        if argv.is_empty() || argv[0].is_empty() {
            return Err(ShellError::SchemaInvalid("argv must be non-empty"));
        }
        self.runner.run(&ShellRequest {
            argv,
            cwd: cwd.into(),
        })
    }
}

/// Deterministic runner (no process) — mirrors sandbox `stub` for unit tests.
#[derive(Clone, Debug)]
pub struct StubShellRunner {
    root: PathBuf,
}

impl StubShellRunner {
    /// # Errors
    /// Returns [`ShellError::SchemaInvalid`] when `root` is not absolute.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ShellError> {
        let root = lexical_normalize(root.as_ref());
        if root.as_os_str().is_empty() || !root.is_absolute() {
            return Err(ShellError::SchemaInvalid("jail root must be absolute"));
        }
        Ok(Self { root })
    }
}

impl ShellRunner for StubShellRunner {
    fn run(&self, req: &ShellRequest) -> Result<ShellResult, ShellError> {
        let _cwd = resolve_under(&self.root, &req.cwd)?;
        Ok(ShellResult {
            exit_code: 0,
            stdout: format!("stub-shell:{}", req.argv.join("\u{1f}")),
            stderr: String::new(),
        })
    }
}

/// Host process runner with lexical cwd jail (sandbox `none` posture).
#[derive(Clone, Debug)]
pub struct HostShellRunner {
    root: PathBuf,
}

impl HostShellRunner {
    /// # Errors
    /// Returns [`ShellError::SchemaInvalid`] when `root` is not absolute.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ShellError> {
        let root = lexical_normalize(root.as_ref());
        if root.as_os_str().is_empty() || !root.is_absolute() {
            return Err(ShellError::SchemaInvalid("jail root must be absolute"));
        }
        Ok(Self { root })
    }
}

impl ShellRunner for HostShellRunner {
    fn run(&self, req: &ShellRequest) -> Result<ShellResult, ShellError> {
        let program = req
            .argv
            .first()
            .filter(|s| !s.is_empty())
            .ok_or(ShellError::SchemaInvalid("argv must be non-empty"))?;
        let cwd = resolve_under(&self.root, &req.cwd)?;
        let mut cmd = Command::new(program);
        if req.argv.len() > 1 {
            cmd.args(&req.argv[1..]);
        }
        cmd.current_dir(&cwd);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Avoid flashing a console window for short host-jail execs (sak204).
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let output = cmd.output().map_err(|e| ShellError::Spawn(e.to_string()))?;
        Ok(ShellResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Registry seed for `tools.shell`.
#[must_use]
pub fn shell_tool_spec() -> ToolSpec {
    ToolSpec {
        id: "tools.shell".into(),
        description: "Run a command via the sandbox shell runner".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "argv": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1
                },
                "cwd": { "type": "string", "default": "." }
            },
            "required": ["argv"]
        }),
    }
}

fn resolve_under(root: &Path, user_path: &str) -> Result<PathBuf, ShellError> {
    if user_path.is_empty() {
        return Err(ShellError::SchemaInvalid("cwd empty"));
    }
    let user = Path::new(user_path);
    let candidate = if user.is_absolute() {
        lexical_normalize(user)
    } else {
        lexical_normalize(&root.join(user))
    };
    if candidate == root || candidate.starts_with(root) {
        Ok(candidate)
    } else {
        Err(ShellError::Escape)
    }
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(comp.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolRegistry;

    fn stub_tools() -> (tempfile::TempDir, ShellTools<StubShellRunner>) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tools = ShellTools::new(StubShellRunner::new(tmp.path()).expect("stub"));
        (tmp, tools)
    }

    #[test]
    fn stub_exec_echoes_argv() {
        let (_tmp, tools) = stub_tools();
        let out = tools
            .exec(vec!["echo".into(), "hi".into()], ".")
            .expect("exec");
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout, "stub-shell:echo\u{1f}hi");
    }

    #[test]
    fn stub_rejects_parent_cwd() {
        let (_tmp, tools) = stub_tools();
        assert_eq!(
            tools
                .exec(vec!["true".into()], "..")
                .expect_err("esc")
                .to_error_code(),
            ErrorCode::SandboxViolation
        );
    }

    #[test]
    fn host_echo_in_jail() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tools = ShellTools::new(HostShellRunner::new(tmp.path()).expect("runner"));
        let req = if cfg!(windows) {
            (vec!["cmd".into(), "/C".into(), "echo hello".into()], ".")
        } else {
            (vec!["echo".into(), "hello".into()], ".")
        };
        let out = tools.exec(req.0, req.1).expect("exec");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.to_lowercase().contains("hello"));
    }

    #[test]
    fn host_cwd_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let tools = ShellTools::new(HostShellRunner::new(tmp.path()).expect("runner"));
        assert_eq!(
            tools
                .exec(vec!["echo".into(), "x".into()], "..")
                .expect_err("esc"),
            ShellError::Escape
        );
    }

    #[test]
    fn empty_argv_rejected() {
        let (_tmp, tools) = stub_tools();
        assert_eq!(
            tools.exec(vec![], ".").expect_err("empty").to_error_code(),
            ErrorCode::SchemaInvalid
        );
    }

    #[test]
    fn shell_spec_registers() {
        let mut reg = ToolRegistry::new();
        reg.register(shell_tool_spec()).expect("reg");
        reg.validate_args("tools.shell", &serde_json::json!({"argv": ["echo", "x"]}))
            .expect("ok");
    }
}
