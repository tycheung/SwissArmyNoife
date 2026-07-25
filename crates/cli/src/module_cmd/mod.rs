//! CLI `module` subcommands.

mod archive;
mod invoke;

use std::path::PathBuf;
use std::process::ExitCode;

use module_registry::{
    install_and_pin, install_from_registry, install_tarball_and_pin, list_installed,
    remove_and_unpin, update_from_path, HttpRegistryClient,
};

use self::archive::is_archive;
use self::invoke::invoke_add;

pub(crate) fn run(mut args: impl Iterator<Item = String>) -> ExitCode {
    match args.next().as_deref() {
        Some("install") => cmd_install(args),
        Some("update") => cmd_update(args),
        Some("remove") => cmd_remove(args),
        Some("list") => cmd_list(),
        Some("invoke") => cmd_invoke(args),
        Some(other) => {
            eprintln!("unknown module subcommand: {other}");
            eprintln!("usage: sak module install|update|remove|list|invoke");
            ExitCode::from(2)
        }
        None => {
            eprintln!("usage: sak module install|update|remove|list|invoke");
            ExitCode::from(2)
        }
    }
}

fn cmd_install(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(first) = args.next() else {
        eprintln!("usage: sak module install <package-dir|archive.tgz>");
        eprintln!("       sak module install --registry <base-url> <id> [version]");
        return ExitCode::from(2);
    };
    if first == "--registry" {
        return cmd_install_registry(args);
    }
    let path = PathBuf::from(first);
    let result = if is_archive(&path) {
        install_tarball_and_pin(&path)
    } else {
        install_and_pin(&path, "path")
    };
    match result {
        Ok(installed) => {
            println!(
                "installed {}@{} -> {}",
                installed.manifest.id,
                installed.manifest.version,
                installed.root.display()
            );
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("module install failed: {code}");
            ExitCode::from(1)
        }
    }
}

fn cmd_install_registry(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(base) = args.next() else {
        eprintln!("usage: sak module install --registry <base-url> <id> [version]");
        return ExitCode::from(2);
    };
    let Some(id) = args.next() else {
        eprintln!("usage: sak module install --registry <base-url> <id> [version]");
        return ExitCode::from(2);
    };
    let version = args.next().unwrap_or_default();
    let client = HttpRegistryClient::new(base);
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio")
        .block_on(install_from_registry(&client, &id, &version));
    match result {
        Ok(installed) => {
            println!(
                "installed {}@{} -> {} (registry)",
                installed.manifest.id,
                installed.manifest.version,
                installed.root.display()
            );
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("module install --registry failed: {code}");
            ExitCode::from(1)
        }
    }
}

fn cmd_update(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(path) = args.next() else {
        eprintln!("usage: sak module update <package-dir>");
        return ExitCode::from(2);
    };
    match update_from_path(&PathBuf::from(path)) {
        Ok(installed) => {
            println!(
                "updated {}@{}",
                installed.manifest.id, installed.manifest.version
            );
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("module update failed: {code}");
            ExitCode::from(1)
        }
    }
}

fn cmd_remove(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(id) = args.next() else {
        eprintln!("usage: sak module remove <id> [version]");
        return ExitCode::from(2);
    };
    let version = args.next();
    match remove_and_unpin(&id, version.as_deref()) {
        Ok(()) => {
            println!("removed {id}");
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("module remove failed: {code}");
            ExitCode::from(1)
        }
    }
}

fn cmd_list() -> ExitCode {
    match list_installed() {
        Ok(items) => {
            if items.is_empty() {
                println!("(no modules installed)");
            } else {
                for m in items {
                    println!(
                        "{}@{} ({}) {}",
                        m.manifest.id,
                        m.manifest.version,
                        m.manifest.runtime.as_str(),
                        m.root.display()
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("module list failed: {code}");
            ExitCode::from(1)
        }
    }
}

fn cmd_invoke(mut args: impl Iterator<Item = String>) -> ExitCode {
    let Some(id) = args.next() else {
        eprintln!("usage: sak module invoke <id> <a> <b>");
        return ExitCode::from(2);
    };
    let Some(a) = args.next().and_then(|s| s.parse::<i32>().ok()) else {
        eprintln!("usage: sak module invoke <id> <a> <b>");
        return ExitCode::from(2);
    };
    let Some(b) = args.next().and_then(|s| s.parse::<i32>().ok()) else {
        eprintln!("usage: sak module invoke <id> <a> <b>");
        return ExitCode::from(2);
    };
    match invoke_add(&id, a, b) {
        Ok(sum) => {
            println!("{sum}");
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("module invoke failed: {code}");
            ExitCode::from(1)
        }
    }
}
