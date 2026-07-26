//! Unified conformance pack runner (`sak529-a`).
//!
//! One command: MCP conformance fixtures + offer golden crates.

use std::process::{Command, ExitStatus};

/// Cargo integration-test targets that constitute the conformance pack.
pub const PACK: &[(&str, &str)] = &[
    ("mcp", "conformance_fixture"),
    ("offer-llm", "golden_llm_routing"),
    ("offer-sandbox", "golden_sandbox"),
    ("offer-memory", "golden_memory"),
    ("offer-egress", "golden_egress"),
    ("offer-eval", "golden_eval"),
    ("types", "offer_fixtures"),
];

/// Run the pack from the workspace root (`SwissArmyNoife/`).
///
/// # Errors
/// Returns a description when a cargo invocation fails to spawn or exits non-zero.
pub fn run_pack() -> Result<(), String> {
    let mut failed = Vec::new();
    for &(pkg, test) in PACK {
        println!("conformance: cargo test -p {pkg} --test {test}");
        let status = cargo_test(pkg, test)?;
        if !status.success() {
            failed.push(format!("{pkg} --test {test}"));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("failed: {}", failed.join("; ")))
    }
}

fn cargo_test(pkg: &str, test: &str) -> Result<ExitStatus, String> {
    let status = Command::new("cargo")
        .args(["test", "-p", pkg, "--test", test, "--", "--quiet"])
        .status()
        .map_err(|e| format!("spawn cargo test -p {pkg}: {e}"))?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_lists_mcp_and_offer_goldens() {
        assert!(PACK
            .iter()
            .any(|(p, t)| *p == "mcp" && *t == "conformance_fixture"));
        assert!(PACK.iter().any(|(p, _)| *p == "offer-llm"));
        assert!(PACK.len() >= 5);
    }
}
