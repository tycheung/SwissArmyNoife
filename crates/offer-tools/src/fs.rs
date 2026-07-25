//! Jailed filesystem tools: read / write / edit / grep.

use std::fs;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;
use types::ErrorCode;

use crate::registry::ToolSpec;

/// Filesystem tool errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FsError {
    #[error("sandbox.violation: path escapes jail root")]
    Escape,
    #[error("schema.invalid: {0}")]
    SchemaInvalid(&'static str),
    #[error("schema.invalid: {0}")]
    NotFound(String),
    #[error("schema.invalid: {0}")]
    EditFailed(String),
    #[error("io: {0}")]
    Io(String),
}

impl FsError {
    #[must_use]
    pub const fn to_error_code(&self) -> ErrorCode {
        match self {
            Self::Escape => ErrorCode::SandboxViolation,
            Self::SchemaInvalid(_) | Self::NotFound(_) | Self::EditFailed(_) => {
                ErrorCode::SchemaInvalid
            }
            Self::Io(_) => ErrorCode::ProviderUnreachable,
        }
    }
}

impl From<std::io::Error> for FsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

/// One grep match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepHit {
    pub line: u32,
    pub text: String,
}

/// How much of a file to return from [`FsTools::read_mode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadMode {
    /// Full UTF-8 contents.
    Full,
    /// First ~40 non-empty lines (structure skim).
    Outline,
    /// First 2 KiB of text (quick digest).
    Digest,
}

fn outline_lines(body: &str, max_lines: usize) -> String {
    body.lines()
        .filter(|l| !l.trim().is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Workspace-scoped filesystem ops (lexical jail, same rules as sandbox jail).
#[derive(Clone, Debug)]
pub struct FsTools {
    root: PathBuf,
}

impl FsTools {
    /// # Errors
    /// Returns [`FsError::SchemaInvalid`] when `root` is empty or relative.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, FsError> {
        let root = lexical_normalize(root.as_ref());
        if root.as_os_str().is_empty() {
            return Err(FsError::SchemaInvalid("jail root empty"));
        }
        if !root.is_absolute() {
            return Err(FsError::SchemaInvalid("jail root must be absolute"));
        }
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read UTF-8 file contents.
    ///
    /// # Errors
    /// Jail escape, missing file, or I/O.
    pub fn read(&self, path: &str) -> Result<String, FsError> {
        let abs = self.resolve(path)?;
        fs::read_to_string(&abs).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FsError::NotFound(path.to_owned())
            } else {
                FsError::Io(e.to_string())
            }
        })
    }

    /// Read with [`ReadMode`] truncation for outline/digest.
    ///
    /// # Errors
    /// Same as [`Self::read`].
    pub fn read_mode(&self, path: &str, mode: ReadMode) -> Result<String, FsError> {
        let body = self.read(path)?;
        Ok(match mode {
            ReadMode::Full => body,
            ReadMode::Outline => outline_lines(&body, 40),
            ReadMode::Digest => body.chars().take(2048).collect(),
        })
    }

    /// Write UTF-8 contents (creates parent dirs).
    ///
    /// # Errors
    /// Jail escape or I/O.
    pub fn write(&self, path: &str, content: &str) -> Result<(), FsError> {
        let abs = self.resolve(path)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs, content)?;
        Ok(())
    }

    /// Replace the first exact occurrence of `old` with `new` (must appear exactly once).
    ///
    /// # Errors
    /// Jail escape, missing file, or `old` missing/ambiguous.
    pub fn edit(&self, path: &str, old: &str, new: &str) -> Result<(), FsError> {
        if old.is_empty() {
            return Err(FsError::SchemaInvalid("old must be non-empty"));
        }
        let body = self.read(path)?;
        let count = body.matches(old).count();
        if count == 0 {
            return Err(FsError::EditFailed("old string not found".into()));
        }
        if count > 1 {
            return Err(FsError::EditFailed("old string not unique".into()));
        }
        let updated = body.replacen(old, new, 1);
        self.write(path, &updated)
    }

    /// Line-oriented substring search (case-sensitive).
    ///
    /// # Errors
    /// Jail escape or I/O.
    pub fn grep(&self, path: &str, pattern: &str) -> Result<Vec<GrepHit>, FsError> {
        if pattern.is_empty() {
            return Err(FsError::SchemaInvalid("pattern must be non-empty"));
        }
        let body = self.read(path)?;
        let mut hits = Vec::new();
        for (idx, line) in body.lines().enumerate() {
            if line.contains(pattern) {
                hits.push(GrepHit {
                    line: u32::try_from(idx + 1).unwrap_or(u32::MAX),
                    text: line.to_owned(),
                });
            }
        }
        Ok(hits)
    }

    fn resolve(&self, user_path: &str) -> Result<PathBuf, FsError> {
        if user_path.is_empty() {
            return Err(FsError::SchemaInvalid("path empty"));
        }
        let user_path = Path::new(user_path);
        let candidate = if user_path.is_absolute() {
            lexical_normalize(user_path)
        } else {
            lexical_normalize(&self.root.join(user_path))
        };
        if candidate == self.root || candidate.starts_with(&self.root) {
            Ok(candidate)
        } else {
            Err(FsError::Escape)
        }
    }
}

/// Seed specs for `tools.fs.*` registry entries.
#[must_use]
pub fn fs_tool_specs() -> Vec<ToolSpec> {
    let obj = |required: &[&str], props: serde_json::Value| {
        serde_json::json!({
            "type": "object",
            "properties": props,
            "required": required,
        })
    };
    vec![
        ToolSpec {
            id: "tools.fs.read".into(),
            description: "Read a UTF-8 file under the workspace jail".into(),
            input_schema: obj(&["path"], serde_json::json!({"path": {"type": "string"}})),
        },
        ToolSpec {
            id: "tools.fs.write".into(),
            description: "Write a UTF-8 file under the workspace jail".into(),
            input_schema: obj(
                &["path", "content"],
                serde_json::json!({
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                }),
            ),
        },
        ToolSpec {
            id: "tools.fs.edit".into(),
            description: "Replace a unique substring in a jailed file".into(),
            input_schema: obj(
                &["path", "old", "new"],
                serde_json::json!({
                    "path": {"type": "string"},
                    "old": {"type": "string"},
                    "new": {"type": "string"}
                }),
            ),
        },
        ToolSpec {
            id: "tools.fs.grep".into(),
            description: "Substring search in a jailed file".into(),
            input_schema: obj(
                &["path", "pattern"],
                serde_json::json!({
                    "path": {"type": "string"},
                    "pattern": {"type": "string"}
                }),
            ),
        },
    ]
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

    fn tools() -> (tempfile::TempDir, FsTools) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fs = FsTools::new(tmp.path()).expect("fs");
        (tmp, fs)
    }

    #[test]
    fn read_write_roundtrip() {
        let (_tmp, fs) = tools();
        fs.write("a/b.txt", "hello").expect("write");
        assert_eq!(fs.read("a/b.txt").expect("read"), "hello");
    }

    #[test]
    fn edit_unique_replace() {
        let (_tmp, fs) = tools();
        fs.write("f.txt", "one two one").expect("write");
        assert!(fs.edit("f.txt", "two", "2").is_ok());
        assert_eq!(fs.read("f.txt").expect("read"), "one 2 one");
        assert!(matches!(
            fs.edit("f.txt", "one", "1"),
            Err(FsError::EditFailed(_))
        ));
    }

    #[test]
    fn grep_finds_lines() {
        let (_tmp, fs) = tools();
        fs.write("g.txt", "alpha\nbeta\nalpha2\n").expect("write");
        let hits = fs.grep("g.txt", "alpha").expect("grep");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[1].line, 3);
    }

    #[test]
    fn escape_is_violation() {
        let (_tmp, fs) = tools();
        assert_eq!(fs.read("../secret").expect_err("esc"), FsError::Escape);
        assert_eq!(
            fs.write("../x", "no").expect_err("esc").to_error_code(),
            ErrorCode::SandboxViolation
        );
    }

    #[test]
    fn read_mode_outline_and_digest() {
        use std::fmt::Write as _;
        let (_tmp, fs) = tools();
        let mut body = String::new();
        for i in 0..60 {
            let _ = write!(body, "line{i}\n\n");
        }
        body.push_str(&"z".repeat(3000));
        fs.write("big.txt", &body).expect("write");
        let outline = fs.read_mode("big.txt", ReadMode::Outline).expect("o");
        assert!(outline.lines().count() <= 40);
        let digest = fs.read_mode("big.txt", ReadMode::Digest).expect("d");
        assert!(digest.chars().count() <= 2048);
        assert_eq!(fs.read_mode("big.txt", ReadMode::Full).expect("f"), body);
    }

    #[test]
    fn specs_register_and_validate() {
        let mut reg = ToolRegistry::new();
        for spec in fs_tool_specs() {
            reg.register(spec).expect("register");
        }
        assert_eq!(reg.list().len(), 4);
        reg.validate_args("tools.fs.read", &serde_json::json!({"path": "a"}))
            .expect("ok");
        assert_eq!(
            reg.validate_args("tools.fs.read", &serde_json::json!({})),
            Err(ErrorCode::SchemaInvalid)
        );
    }
}
