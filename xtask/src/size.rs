//! Source file line-count smoke gate (`sak011-a`).

use std::fs;
use std::path::{Path, PathBuf};

pub const WARN_LOC: usize = 800;
pub const FAIL_LOC: usize = 1000;

/// Count lines in a Rust source file (including blank lines).
pub fn max_rs_loc(path: &Path) -> std::io::Result<usize> {
    let text = fs::read_to_string(path)?;
    Ok(text.lines().count())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileReport {
    pub rel_path: String,
    pub lines: usize,
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn scan_crate_sources(root: &Path) -> std::io::Result<Vec<FileReport>> {
    let crates_dir = root.join("crates");
    let mut reports = Vec::new();
    let Ok(entries) = fs::read_dir(&crates_dir) else {
        return Ok(reports);
    };
    for entry in entries.flatten() {
        let src = entry.path().join("src");
        if !src.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        collect_rs_files(&src, &mut files);
        for path in files {
            let lines = max_rs_loc(&path)?;
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            reports.push(FileReport { rel_path, lines });
        }
    }
    reports.sort_by(|a, b| {
        b.lines
            .cmp(&a.lines)
            .then_with(|| a.rel_path.cmp(&b.rel_path))
    });
    Ok(reports)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask parent")
        .to_path_buf()
}

/// Scan `crates/*/src/**/*.rs`. Returns hard failures (> `FAIL_LOC`) or Ok(warnings 800–1000).
pub fn check_workspace() -> Result<Vec<FileReport>, Vec<FileReport>> {
    let root = workspace_root();
    let reports = scan_crate_sources(&root).map_err(|err| {
        vec![FileReport {
            rel_path: format!("scan error: {err}"),
            lines: 0,
        }]
    })?;

    let mut failures = Vec::new();
    let mut warnings = Vec::new();
    for report in reports {
        if report.lines > FAIL_LOC {
            failures.push(report);
        } else if report.lines >= WARN_LOC {
            warnings.push(report);
        }
    }
    if failures.is_empty() {
        Ok(warnings)
    } else {
        Err(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_rs_loc_counts_lines() {
        let dir = std::env::temp_dir().join(format!("xtask-size-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("sample.rs");
        fs::write(&path, "fn main() {\n\n}\n").expect("write");
        assert_eq!(max_rs_loc(&path).expect("read"), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_under_hard_limit() {
        check_workspace().expect("no crate src file should exceed FAIL_LOC");
    }
}
