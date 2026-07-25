//! MCP helpers for jailed `tools.fs` / `tools.shell`.

use offer_tools::{FsTools, HostShellRunner, ReadMode, ShellTools};
use rmcp::ErrorData as McpError;

pub(crate) fn boot_fs_shell() -> Result<(FsTools, ShellTools<HostShellRunner>), McpError> {
    let jail = env::config_dir().join("sandbox-jail");
    std::fs::create_dir_all(&jail)
        .map_err(|e| McpError::internal_error(format!("jail mkdir: {e}"), None))?;
    let fs = FsTools::new(&jail)
        .map_err(|e| McpError::internal_error(format!("fs tools: {e}"), None))?;
    let runner = HostShellRunner::new(&jail)
        .map_err(|e| McpError::internal_error(format!("shell runner: {e}"), None))?;
    Ok((fs, ShellTools::new(runner)))
}

pub(crate) fn parse_read_mode(raw: Option<&str>) -> Result<ReadMode, McpError> {
    match raw.unwrap_or("full").to_ascii_lowercase().as_str() {
        "full" => Ok(ReadMode::Full),
        "outline" => Ok(ReadMode::Outline),
        "digest" => Ok(ReadMode::Digest),
        other => Err(McpError::invalid_params(
            format!("schema.invalid: unknown read mode {other}"),
            None,
        )),
    }
}

pub(crate) fn mode_label(mode: ReadMode) -> &'static str {
    match mode {
        ReadMode::Full => "full",
        ReadMode::Outline => "outline",
        ReadMode::Digest => "digest",
    }
}

pub(crate) fn fs_err(e: &offer_tools::FsError) -> McpError {
    McpError::invalid_params(format!("{}: {e}", e.to_error_code()), None)
}

pub(crate) fn shell_err(e: &offer_tools::ShellError) -> McpError {
    McpError::invalid_params(format!("{}: {e}", e.to_error_code()), None)
}
