//! Bubblewrap sandbox backend (`bwrap`; Linux-only exec).

use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

use crate::backend::{
    reject_outside_argv_paths, validate_argv, ExecRequest, ExecResult, SandboxBackend, SandboxError,
};
#[cfg(unix)]
use crate::backend::{wait_output_or_timeout, DEFAULT_EXEC_TIMEOUT};
use crate::{FilesystemJail, JailError};

const GUEST_ROOT: &str = "/sak";

/// `bwrap` exec: bind the jail root at `/sak` and unshare the network namespace.
#[derive(Clone, Debug)]
pub struct BwrapBackend {
    jail: FilesystemJail,
    #[cfg_attr(not(unix), allow(dead_code))]
    bwrap_bin: PathBuf,
}

impl BwrapBackend {
    #[must_use]
    pub fn new(jail: FilesystemJail) -> Self {
        Self {
            jail,
            bwrap_bin: PathBuf::from("bwrap"),
        }
    }

    /// # Errors
    /// Propagates [`JailError`] when the root is invalid.
    pub fn with_root(root: impl AsRef<Path>) -> Result<Self, JailError> {
        Ok(Self::new(FilesystemJail::new(root)?))
    }

    #[must_use]
    pub fn jail(&self) -> &FilesystemJail {
        &self.jail
    }

    /// Build `bwrap …` argv excluding the binary name.
    ///
    /// # Errors
    /// Empty argv or jail escape.
    pub fn build_run_args(&self, req: &ExecRequest) -> Result<Vec<String>, SandboxError> {
        let _program = validate_argv(&req.argv)?;
        reject_outside_argv_paths(self.jail(), &req.argv)?;
        let cwd = self.jail.resolve(&req.cwd)?;
        let workdir = guest_workdir(self.jail.root(), &cwd)?;
        let mut args = vec![
            "--die-with-parent".into(),
            "--unshare-net".into(),
            "--unshare-uts".into(),
            "--bind".into(),
            self.jail.root().to_string_lossy().into_owned(),
            GUEST_ROOT.into(),
            "--chdir".into(),
            workdir,
            "--".into(),
        ];
        args.extend(req.argv.iter().cloned());
        Ok(args)
    }
}

impl SandboxBackend for BwrapBackend {
    fn exec(&self, req: &ExecRequest) -> Result<ExecResult, SandboxError> {
        #[cfg(not(unix))]
        {
            let _ = req;
            Err(SandboxError::Spawn(
                "bwrap is Linux-only (SANDBOX_BACKEND=bwrap)".into(),
            ))
        }
        #[cfg(unix)]
        {
            let args = self.build_run_args(req)?;
            let mut cmd = Command::new(&self.bwrap_bin);
            cmd.args(&args);
            wait_output_or_timeout(cmd, DEFAULT_EXEC_TIMEOUT).map_err(|e| match e {
                SandboxError::Spawn(msg) => SandboxError::Spawn(format!("bwrap: {msg}")),
                other => other,
            })
        }
    }
}

fn guest_workdir(root: &Path, cwd: &Path) -> Result<String, SandboxError> {
    let rel = cwd.strip_prefix(root).map_err(|_| {
        SandboxError::Violation("path_escape", "resolved cwd is not under jail root".into())
    })?;
    if rel.as_os_str().is_empty() {
        return Ok(GUEST_ROOT.into());
    }
    let rel = rel.to_string_lossy().replace('\\', "/");
    Ok(format!("{GUEST_ROOT}/{rel}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::ErrorCode;

    fn backend() -> (tempfile::TempDir, BwrapBackend) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backend = BwrapBackend::with_root(tmp.path()).expect("backend");
        (tmp, backend)
    }

    #[test]
    fn build_run_args_shape() {
        let (_tmp, backend) = backend();
        let args = backend
            .build_run_args(&ExecRequest {
                argv: vec!["echo".into(), "hi".into()],
                cwd: PathBuf::from("."),
            })
            .expect("args");
        assert!(args.iter().any(|a| a == "--die-with-parent"));
        assert!(args.iter().any(|a| a == "--unshare-net"));
        assert!(args.iter().any(|a| a == "--unshare-uts"));
        assert!(args.iter().any(|a| a == "--bind"));
        assert!(args.iter().any(|a| a == GUEST_ROOT));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--chdir" && w[1] == GUEST_ROOT));
        assert_eq!(args[args.len() - 3], "--");
        assert_eq!(args[args.len() - 2], "echo");
        assert_eq!(args[args.len() - 1], "hi");
    }

    #[test]
    fn cwd_escape_is_sandbox_violation() {
        let (_tmp, backend) = backend();
        let err = backend
            .build_run_args(&ExecRequest {
                argv: vec!["true".into()],
                cwd: PathBuf::from(".."),
            })
            .expect_err("escape");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }

    #[cfg(windows)]
    #[test]
    fn exec_is_linux_only_on_windows() {
        let (_tmp, backend) = backend();
        let err = backend
            .exec(&ExecRequest {
                argv: vec!["echo".into(), "x".into()],
                cwd: PathBuf::from("."),
            })
            .expect_err("windows");
        assert!(err.to_string().contains("Linux-only"));
    }
}
