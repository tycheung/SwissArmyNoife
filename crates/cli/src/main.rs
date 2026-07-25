//! `SwissArmyNoife` CLI entrypoint (`sak` / `swissarmynoife`).

mod module_cmd;

use std::process::ExitCode;

use types::core_schema_document;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("hello") | None => {
            println!("sak {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("schema") => match args.next().as_deref() {
            Some("dump") | None => match serde_json::to_string_pretty(&core_schema_document()) {
                Ok(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("schema dump failed: {err}");
                    ExitCode::from(1)
                }
            },
            Some("tools") => match serde_json::to_string_pretty(&mcp::tool_input_schemas()) {
                Ok(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("schema tools failed: {err}");
                    ExitCode::from(1)
                }
            },
            Some(other) => {
                eprintln!("unknown schema subcommand: {other}");
                eprintln!("usage: sak schema dump | sak schema tools");
                ExitCode::from(2)
            }
        },
        Some("module") => module_cmd::run(args),
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: sak [hello | schema dump|tools | module …]");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn package_version_is_semverish() {
        let v = env!("CARGO_PKG_VERSION");
        assert!(v.split('.').count() >= 2, "version={v}");
    }

    #[test]
    fn tool_schemas_include_bind() {
        let doc = mcp::tool_input_schemas();
        assert!(doc["tools"]["bind"].is_object());
        assert!(doc["tools"]["broker_health"].is_object());
    }
}
