//! Docker sandbox backend (`docker run` + jail volume mount).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::{validate_argv, ExecRequest, ExecResult, SandboxBackend, SandboxError};
use crate::mount_policy::{MountPolicyError, WorkspaceMountPolicy};
use crate::{FilesystemJail, JailError};

const CONTAINER_ROOT: &str = "/sak";
const DEFAULT_IMAGE: &str = "alpine:3.20";

/// Docker-backed exec: mounts the jail root and runs argv in the container.
#[derive(Clone, Debug)]
pub struct DockerBackend {
    jail: FilesystemJail,
    image: String,
    docker_bin: PathBuf,
    mount_policy: WorkspaceMountPolicy,
}

impl DockerBackend {
    /// Jail + image (default binary name `docker`).
    #[must_use]
    pub fn new(jail: FilesystemJail, image: impl Into<String>) -> Self {
        Self {
            jail,
            image: image.into(),
            docker_bin: PathBuf::from("docker"),
            mount_policy: WorkspaceMountPolicy::default(),
        }
    }

    /// Attach workspace bind-mount policy (validated before `docker run`).
    #[must_use]
    pub fn with_mount_policy(mut self, policy: WorkspaceMountPolicy) -> Self {
        self.mount_policy = policy;
        self
    }

    #[must_use]
    pub fn mount_policy(&self) -> &WorkspaceMountPolicy {
        &self.mount_policy
    }

    /// Build with default image `alpine:3.20`.
    ///
    /// # Errors
    /// Propagates [`JailError`] when the root is invalid.
    pub fn with_root(root: impl AsRef<Path>) -> Result<Self, JailError> {
        Ok(Self::new(FilesystemJail::new(root)?, DEFAULT_IMAGE))
    }

    #[must_use]
    pub fn jail(&self) -> &FilesystemJail {
        &self.jail
    }

    #[must_use]
    pub fn image(&self) -> &str {
        &self.image
    }

    /// Build `docker run …` argv (program + args, excluding the docker binary).
    ///
    /// # Errors
    /// Returns [`SandboxError`] on empty argv or jail escape.
    pub fn build_run_args(&self, req: &ExecRequest) -> Result<Vec<String>, SandboxError> {
        let _program = validate_argv(&req.argv)?;
        let cwd = self.jail.resolve(&req.cwd)?;
        let workdir = container_workdir(self.jail.root(), &cwd)?;
        self.mount_policy
            .validate()
            .map_err(|e| mount_policy_to_sandbox_error(&e))?;

        let volume = format!("{}:{CONTAINER_ROOT}", self.jail.root().to_string_lossy());
        let mut args = vec!["run".into(), "--rm".into(), "-v".into(), volume];
        for mount in &self.mount_policy.mounts {
            let guest = container_guest_path(&mount.guest);
            let mut spec = format!("{}:{guest}", mount.host.to_string_lossy());
            if mount.read_only {
                spec.push_str(":ro");
            }
            args.push("-v".into());
            args.push(spec);
        }
        args.extend(["-w".into(), workdir, self.image.clone()]);
        args.extend(req.argv.iter().cloned());
        Ok(args)
    }
}

fn mount_policy_to_sandbox_error(err: &MountPolicyError) -> SandboxError {
    match err {
        MountPolicyError::Escape => {
            SandboxError::Violation("path_escape", "guest path escapes jail root".into())
        }
        MountPolicyError::SchemaInvalid(msg) => SandboxError::SchemaInvalid(msg),
    }
}

fn container_guest_path(guest: &Path) -> String {
    let rel = guest.to_string_lossy().replace('\\', "/");
    format!("{CONTAINER_ROOT}/{rel}")
}

impl SandboxBackend for DockerBackend {
    fn exec(&self, req: &ExecRequest) -> Result<ExecResult, SandboxError> {
        let args = self.build_run_args(req)?;
        let output = Command::new(&self.docker_bin)
            .args(&args)
            .output()
            .map_err(|e| SandboxError::Spawn(format!("docker: {e}")))?;
        let exit_code = output.status.code().unwrap_or(-1);
        Ok(ExecResult {
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn container_workdir(root: &Path, cwd: &Path) -> Result<String, SandboxError> {
    let rel = cwd.strip_prefix(root).map_err(|_| {
        SandboxError::Violation("path_escape", "resolved cwd is not under jail root".into())
    })?;
    if rel.as_os_str().is_empty() {
        return Ok(CONTAINER_ROOT.into());
    }
    let rel = rel.to_string_lossy().replace('\\', "/");
    Ok(format!("{CONTAINER_ROOT}/{rel}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::ErrorCode;

    fn backend() -> (tempfile::TempDir, DockerBackend) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backend = DockerBackend::with_root(tmp.path()).expect("backend");
        (tmp, backend)
    }

    fn docker_available() -> bool {
        Command::new("docker")
            .args(["info"])
            .output()
            .is_ok_and(|o| o.status.success())
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
        assert_eq!(args[0], "run");
        assert!(args.iter().any(|a| a == "--rm"));
        assert!(args.iter().any(|a| a == "-v"));
        assert!(args.iter().any(|a| a == "-w"));
        assert!(args.iter().any(|a| a == CONTAINER_ROOT));
        assert!(args.iter().any(|a| a == DEFAULT_IMAGE));
        assert_eq!(args[args.len() - 2], "echo");
        assert_eq!(args[args.len() - 1], "hi");
    }

    #[test]
    fn build_run_args_nested_workdir() {
        let (_tmp, backend) = backend();
        let args = backend
            .build_run_args(&ExecRequest {
                argv: vec!["true".into()],
                cwd: PathBuf::from("sub/dir"),
            })
            .expect("args");
        let w = args
            .iter()
            .position(|a| a == "-w")
            .map(|i| args[i + 1].as_str())
            .expect("-w");
        assert_eq!(w, "/sak/sub/dir");
    }

    #[test]
    fn mount_policy_ro_volume_in_args() {
        let (_tmp, backend) = backend();
        let backend = backend.with_mount_policy(WorkspaceMountPolicy {
            mounts: vec![crate::BindMount {
                host: PathBuf::from("/data/project"),
                guest: PathBuf::from("workspace"),
                read_only: true,
            }],
        });
        let args = backend
            .build_run_args(&ExecRequest {
                argv: vec!["true".into()],
                cwd: PathBuf::from("."),
            })
            .expect("args");
        assert!(
            args.windows(2)
                .any(|w| w[0] == "-v" && w[1] == "/data/project:/sak/workspace:ro"),
            "expected ro bind mount in args: {args:?}"
        );
    }

    #[test]
    fn invalid_mount_policy_fails_before_docker() {
        let (_tmp, backend) = backend();
        let backend = backend.with_mount_policy(WorkspaceMountPolicy {
            mounts: vec![crate::BindMount {
                host: PathBuf::from("/data"),
                guest: PathBuf::from("../escape"),
                read_only: false,
            }],
        });
        let err = backend
            .build_run_args(&ExecRequest {
                argv: vec!["true".into()],
                cwd: PathBuf::from("."),
            })
            .expect_err("invalid policy");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }

    #[test]
    fn cwd_escape_before_docker() {
        let (_tmp, backend) = backend();
        let err = backend
            .build_run_args(&ExecRequest {
                argv: vec!["true".into()],
                cwd: PathBuf::from(".."),
            })
            .expect_err("escape");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }

    #[test]
    fn live_docker_echo_or_skip() {
        if !docker_available() {
            eprintln!("sak153: skip live docker test (docker unavailable)");
            return;
        }
        let (_tmp, backend) = backend();
        let out = backend
            .exec(&ExecRequest {
                argv: vec!["echo".into(), "fixture-docker".into()],
                cwd: PathBuf::from("."),
            })
            .expect("docker exec");
        assert_eq!(out.exit_code, 0, "stderr={}", out.stderr);
        assert!(out.stdout.contains("fixture-docker"));
    }
}
