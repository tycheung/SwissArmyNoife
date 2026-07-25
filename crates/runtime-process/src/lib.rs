//! Process runtime loader — secondary to wasm (`sak355`).
//!
//! Spawns the payload entrypoint with one JSON argv; stdout is the result.

use std::path::Path;
use std::process::Command;

use types::ErrorCode;

/// Run a process-backed module: `entrypoint <args_json>` → stdout.
///
/// # Errors
/// Spawn / non-zero exit → [`ErrorCode::ModuleIncompatible`].
pub fn invoke_process_module(entrypoint: &Path, args_json: &str) -> Result<String, ErrorCode> {
    let out = Command::new(entrypoint)
        .arg(args_json)
        .output()
        .map_err(|_| ErrorCode::ModuleIncompatible)?;
    if !out.status.success() {
        return Err(ErrorCode::ModuleIncompatible);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn spawn_echo_script() {
        let tmp = tempfile::tempdir().unwrap();
        #[cfg(windows)]
        {
            let script = tmp.path().join("echo.cmd");
            fs::write(&script, "@echo %~1\r\n").unwrap();
            let out = invoke_process_module(&script, "{\"x\":1}").expect("run");
            assert!(out.contains('1') && out.contains('x'), "out={out}");
        }
        #[cfg(not(windows))]
        {
            let script = tmp.path().join("echo.sh");
            fs::write(&script, "#!/bin/sh\necho \"$1\"\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
            let out = invoke_process_module(&script, "{\"x\":1}").expect("run");
            assert!(out.contains("{\"x\":1}"), "out={out}");
        }
    }
}
