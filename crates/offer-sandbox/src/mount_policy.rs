//! Workspace bind-mount policy types (validation only; Docker wiring is later).

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use types::ErrorCode;

/// Host → guest bind mount entry (frozen on binding policy).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindMount {
    pub host: PathBuf,
    pub guest: PathBuf,
    pub read_only: bool,
}

/// Collection of bind mounts applied inside the sandbox jail namespace.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceMountPolicy {
    pub mounts: Vec<BindMount>,
}

/// Bind-mount policy validation failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MountPolicyError {
    #[error("sandbox.violation:path_escape: guest path escapes jail root")]
    Escape,
    #[error("schema.invalid: {0}")]
    SchemaInvalid(&'static str),
}

impl MountPolicyError {
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::Escape => ErrorCode::SandboxViolation,
            Self::SchemaInvalid(_) => ErrorCode::SchemaInvalid,
        }
    }
}

impl BindMount {
    /// Validate host and guest paths for jail-safe mounting.
    ///
    /// Guest paths must be relative (no absolute paths) and must not `..` above the jail root.
    ///
    /// # Errors
    /// Returns [`MountPolicyError`] when paths are invalid or escape the jail.
    pub fn validate(&self) -> Result<(), MountPolicyError> {
        validate_host_path(&self.host)?;
        validate_guest_path(&self.guest)?;
        Ok(())
    }
}

impl WorkspaceMountPolicy {
    /// Validate every mount in the policy.
    ///
    /// # Errors
    /// Returns the first [`MountPolicyError`] from any mount.
    pub fn validate(&self) -> Result<(), MountPolicyError> {
        for mount in &self.mounts {
            mount.validate()?;
        }
        Ok(())
    }
}

fn validate_host_path(host: &Path) -> Result<(), MountPolicyError> {
    if host.as_os_str().is_empty() {
        return Err(MountPolicyError::SchemaInvalid("host path empty"));
    }
    Ok(())
}

fn validate_guest_path(guest: &Path) -> Result<(), MountPolicyError> {
    if guest.as_os_str().is_empty() {
        return Err(MountPolicyError::SchemaInvalid("guest path empty"));
    }
    if guest.is_absolute() {
        return Err(MountPolicyError::SchemaInvalid(
            "guest path must be relative",
        ));
    }
    let mut depth = 0u32;
    for comp in guest.components() {
        match comp {
            Component::Prefix(_) | Component::RootDir => {
                return Err(MountPolicyError::SchemaInvalid(
                    "guest path must be relative",
                ));
            }
            Component::ParentDir => {
                depth = depth.checked_sub(1).ok_or(MountPolicyError::Escape)?;
            }
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
        }
    }
    if depth == 0 {
        return Err(MountPolicyError::SchemaInvalid("guest path empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn relative_guest_accepted() {
        let m = BindMount {
            host: PathBuf::from("/data/project"),
            guest: PathBuf::from("workspace/src"),
            read_only: false,
        };
        m.validate().expect("ok");
    }

    #[test]
    fn absolute_guest_rejected() {
        let m = BindMount {
            host: PathBuf::from("/data"),
            guest: PathBuf::from("/etc/passwd"),
            read_only: true,
        };
        let err = m.validate().expect_err("abs guest");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }

    #[test]
    fn parent_escape_rejected() {
        let m = BindMount {
            host: PathBuf::from("/data"),
            guest: PathBuf::from("../outside"),
            read_only: false,
        };
        let err = m.validate().expect_err("escape");
        assert_eq!(err, MountPolicyError::Escape);
    }

    #[test]
    fn empty_host_rejected() {
        let m = BindMount {
            host: PathBuf::new(),
            guest: PathBuf::from("work"),
            read_only: false,
        };
        let err = m.validate().expect_err("empty host");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }

    #[test]
    fn policy_validates_all_mounts() {
        let policy = WorkspaceMountPolicy {
            mounts: vec![
                BindMount {
                    host: PathBuf::from("/a"),
                    guest: PathBuf::from("a"),
                    read_only: false,
                },
                BindMount {
                    host: PathBuf::from("/b"),
                    guest: PathBuf::from(".."),
                    read_only: true,
                },
            ],
        };
        let err = policy.validate().expect_err("second mount");
        assert_eq!(err, MountPolicyError::Escape);
    }

    #[test]
    fn serde_roundtrip() {
        let policy = WorkspaceMountPolicy {
            mounts: vec![BindMount {
                host: PathBuf::from("/host"),
                guest: PathBuf::from("guest/dir"),
                read_only: true,
            }],
        };
        let v = serde_json::to_value(&policy).expect("serialize");
        assert_eq!(
            v,
            json!({
                "mounts": [{
                    "host": "/host",
                    "guest": "guest/dir",
                    "read_only": true
                }]
            })
        );
        let back: WorkspaceMountPolicy = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back, policy);
        back.validate().expect("valid after roundtrip");
    }
}
