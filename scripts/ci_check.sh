#!/usr/bin/env bash
# SwissArmyNoife local CI gate (Unix)
# Usage: from SwissArmyNoife/:  ./scripts/ci_check.sh
# Cross-repo matrix (Nimbusware, marketplace-api/web): ../docs/ci-matrix.md
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== cargo fmt --check =="
cargo fmt --all -- --check

echo "== cargo clippy =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== cargo test =="
cargo test --workspace

echo "== xtask boundaries =="
cargo run -q -p xtask -- boundaries

echo "== xtask conformance =="
cargo run -q -p xtask -- conformance

echo "== xtask schema export --check =="
cargo run -q -p xtask -- schema export --check

echo "== cargo deny check licenses =="
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "cargo-deny not found; installing..."
  cargo install cargo-deny --locked
fi
cargo deny check licenses

echo "ci_check: OK"
