//! Maintainer CLI: `cargo run -p xtask -- <cmd>`.

mod boundaries;
mod conformance;
mod schema_export;
mod size;

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("boundaries") => match boundaries::check_workspace() {
            Ok(()) => {
                println!("boundaries: OK");
                ExitCode::SUCCESS
            }
            Err(errors) => {
                eprintln!("boundaries: FAILED");
                for e in errors {
                    eprintln!("  - {e}");
                }
                ExitCode::from(1)
            }
        },
        Some("size") => match size::check_workspace() {
            Ok(warnings) => {
                if warnings.is_empty() {
                    println!("size: OK (no files >= {} LOC)", size::WARN_LOC);
                } else {
                    eprintln!("size: WARN (soft limit {} LOC)", size::WARN_LOC);
                    for w in &warnings {
                        eprintln!("  - {}: {} lines", w.rel_path, w.lines);
                    }
                }
                ExitCode::SUCCESS
            }
            Err(failures) => {
                eprintln!("size: FAILED (hard limit {} LOC)", size::FAIL_LOC);
                for f in &failures {
                    eprintln!("  - {}: {} lines", f.rel_path, f.lines);
                }
                ExitCode::from(1)
            }
        },
        Some("conformance") => match conformance::run_pack() {
            Ok(()) => {
                println!("conformance: OK ({} packs)", conformance::PACK.len());
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("conformance: FAILED — {err}");
                ExitCode::from(1)
            }
        },
        // Schemars emit via `cli schema tools` (`sak530-a`).
        Some("schema") => match args.next().as_deref() {
            Some("export") => {
                let check = args.next().as_deref() == Some("--check");
                if check {
                    match schema_export::check_schemars() {
                        Ok(()) => {
                            println!(
                                "schema export --check: OK ({} canonical tools; schemars)",
                                schema_export::CANONICAL_TOOL_NAMES.len()
                            );
                            ExitCode::SUCCESS
                        }
                        Err(err) => {
                            eprintln!("schema export --check: FAILED — {err}");
                            ExitCode::from(1)
                        }
                    }
                } else {
                    match schema_export::emit_document() {
                        Ok(doc) => {
                            print!("{doc}");
                            ExitCode::SUCCESS
                        }
                        Err(err) => {
                            eprintln!("schema export: FAILED — {err}");
                            ExitCode::from(1)
                        }
                    }
                }
            }
            other => {
                eprintln!("usage: xtask schema export [--check]");
                if let Some(o) = other {
                    eprintln!("unknown schema subcommand: {o}");
                }
                ExitCode::from(2)
            }
        },
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: xtask {{boundaries|size|conformance|schema export [--check]}}");
            ExitCode::from(2)
        }
        None => {
            eprintln!("usage: xtask {{boundaries|size|conformance|schema export [--check]}}");
            ExitCode::from(2)
        }
    }
}
