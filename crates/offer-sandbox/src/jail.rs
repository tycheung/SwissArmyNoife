//! Workspace filesystem jail — lexical containment under a root.

use std::path::{Component, Path, PathBuf};

use thiserror::Error;
use types::ErrorCode;

/// Jail failures (escape attempts and bad roots).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum JailError {
    #[error("sandbox.violation:path_escape: path escapes jail root")]
    Escape,
    #[error("schema.invalid: {0}")]
    SchemaInvalid(&'static str),
}

impl JailError {
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::Escape => ErrorCode::SandboxViolation,
            Self::SchemaInvalid(_) => ErrorCode::SchemaInvalid,
        }
    }
}

/// Restricts resolved paths to a single root directory tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemJail {
    root: PathBuf,
}

impl FilesystemJail {
    /// Create a jail rooted at `root` (must be absolute after lexical normalize).
    ///
    /// # Errors
    /// Returns [`JailError::SchemaInvalid`] when `root` is empty or relative.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, JailError> {
        let root = lexical_normalize(root.as_ref());
        if root.as_os_str().is_empty() {
            return Err(JailError::SchemaInvalid("jail root empty"));
        }
        if !root.is_absolute() {
            return Err(JailError::SchemaInvalid("jail root must be absolute"));
        }
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve `user_path` under the jail.
    ///
    /// Relative paths join the root. Absolute paths are allowed only if they stay inside root.
    ///
    /// # Errors
    /// Returns [`JailError::Escape`] when the normalized path leaves the root.
    pub fn resolve(&self, user_path: impl AsRef<Path>) -> Result<PathBuf, JailError> {
        let user_path = user_path.as_ref();
        if user_path.as_os_str().is_empty() {
            return Err(JailError::SchemaInvalid("path empty"));
        }
        let candidate = if user_path.is_absolute() {
            lexical_normalize(user_path)
        } else {
            lexical_normalize(&self.root.join(user_path))
        };
        if path_is_within(&candidate, &self.root) {
            Ok(candidate)
        } else {
            Err(JailError::Escape)
        }
    }
}

/// Lexical normalize: collapse `.` / `..` without touching the filesystem.
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

fn path_is_within(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn relative_path_resolves_under_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = lexical_normalize(tmp.path());
        let jail = FilesystemJail::new(&root).expect("jail");
        let got = jail.resolve("src/main.rs").expect("ok");
        assert_eq!(got, root.join("src").join("main.rs"));
    }

    #[test]
    fn parent_escape_is_violation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let jail = FilesystemJail::new(tmp.path()).expect("jail");
        let err = jail.resolve("../secret").expect_err("escape");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }

    #[test]
    fn nested_dotdot_stays_inside() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = lexical_normalize(tmp.path());
        let jail = FilesystemJail::new(&root).expect("jail");
        let got = jail.resolve("a/b/../c").expect("ok");
        assert_eq!(got, root.join("a").join("c"));
    }

    #[test]
    fn absolute_outside_is_violation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = lexical_normalize(tmp.path());
        let jail = FilesystemJail::new(&root).expect("jail");
        let outside = if cfg!(windows) {
            PathBuf::from(r"C:\Windows\System32")
        } else {
            PathBuf::from("/etc/passwd")
        };
        if outside.starts_with(&root) {
            return;
        }
        let err = jail.resolve(&outside).expect_err("outside");
        assert_eq!(err, JailError::Escape);
    }

    #[test]
    fn relative_root_rejected() {
        let err = FilesystemJail::new("relative/root").expect_err("rel");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }

    #[test]
    fn empty_path_rejected() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let jail = FilesystemJail::new(tmp.path()).expect("jail");
        let err = jail.resolve("").expect_err("empty");
        assert_eq!(err.to_error_code(), ErrorCode::SchemaInvalid);
    }
}
