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

    /// Resolve `user_path` then follow symlinks; reject if the real path leaves the jail.
    ///
    /// Missing trailing components are allowed when an existing ancestor stays inside.
    ///
    /// # Errors
    /// Returns [`JailError::Escape`] when the real path leaves the root.
    pub fn resolve_canonical(&self, user_path: impl AsRef<Path>) -> Result<PathBuf, JailError> {
        let lexical = self.resolve(user_path)?;
        let canonical_root = match std::fs::canonicalize(&self.root) {
            Ok(p) => strip_verbatim(p),
            Err(_) => self.root.clone(),
        };
        let real = canonicalize_existing_prefix(&lexical)?;
        if path_is_within(&real, &canonical_root) {
            Ok(real)
        } else {
            Err(JailError::Escape)
        }
    }
}

fn canonicalize_existing_prefix(path: &Path) -> Result<PathBuf, JailError> {
    let mut cur = path.to_path_buf();
    let mut missing = Vec::new();
    while !cur.exists() {
        let name = cur
            .file_name()
            .ok_or(JailError::SchemaInvalid("path empty"))?
            .to_os_string();
        missing.push(name);
        if !cur.pop() {
            return Err(JailError::SchemaInvalid("path empty"));
        }
    }
    let mut real = std::fs::canonicalize(&cur).map_err(|_| JailError::Escape)?;
    real = strip_verbatim(real);
    for name in missing.iter().rev() {
        real.push(name);
    }
    Ok(real)
}

/// Drop Windows `\\?\` verbatim prefix so `starts_with` works on canonical paths.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
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

    fn try_symlink(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link).is_ok()
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(target, link).is_ok()
                || std::os::windows::fs::symlink_file(target, link).is_ok()
        }
    }

    #[test]
    fn canonical_inside_file_stays_inside() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let jail = FilesystemJail::new(tmp.path()).expect("jail");
        let file = tmp.path().join("inside.txt");
        std::fs::write(&file, b"ok").expect("write");
        let got = jail.resolve_canonical("inside.txt").expect("ok");
        let expect = strip_verbatim(std::fs::canonicalize(&file).expect("canon"));
        assert_eq!(got, expect);
    }

    #[test]
    fn symlink_escape_is_violation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let jail = FilesystemJail::new(tmp.path()).expect("jail");
        let outside = std::env::temp_dir().join("sak551-outside");
        let _ = std::fs::create_dir_all(&outside);
        let link = tmp.path().join("escape-link");
        if !try_symlink(&outside, &link) {
            eprintln!("sak551-a: skip symlink test (create failed)");
            return;
        }
        let err = jail.resolve_canonical("escape-link").expect_err("escape");
        assert_eq!(err.to_error_code(), ErrorCode::SandboxViolation);
    }

    #[test]
    fn missing_child_under_jail_is_ok() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let jail = FilesystemJail::new(tmp.path()).expect("jail");
        let got = jail.resolve_canonical("not-yet.txt").expect("ok");
        assert!(got.ends_with("not-yet.txt"));
        assert!(path_is_within(&got, tmp.path()) || got.starts_with(tmp.path()));
    }
}
