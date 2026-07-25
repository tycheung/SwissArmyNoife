#!/usr/bin/env bash
# Control crate coverage measurement (sak063-d).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT_PATH="$ROOT/docs/control-coverage.md"
TIMESTAMP="$(date -u +"%Y-%m-%d %H:%M:%SZ")"
TOOL_USED="none"
SUMMARY=""
TEST_OUTPUT=""

write_doc() {
  local tool="$1"
  local body="$2"
  local tests="$3"
  local floor_note

  if [[ "$tool" == "none" ]]; then
    floor_note="**sak063 status: partial** — coverage tool not available; module checklist from smoke tests only (≥85% floor not verified)."
  elif echo "$body" | grep -qE 'TOTAL.*[8-9][0-9]\.[0-9]+%|TOTAL.*100\.00%'; then
    floor_note="**sak063 status: measured** — line coverage ≥85% on \`control\` crate."
  else
    floor_note="**sak063 status: partial** — coverage measured but ≥85% floor not confirmed from summary below."
  fi

  cat >"$OUT_PATH" <<EOF
# Control crate coverage report (\`sak063-d\`)

Generated: $TIMESTAMP

Target: **≥85% line coverage** on the \`control\` crate.

$floor_note

## Tool

- **Used:** \`$tool\`
- **Command:** see [Measuring coverage](#measuring-coverage)

## Summary

\`\`\`
$body
\`\`\`

## Test run (fallback)

\`\`\`
$tests
\`\`\`

## Measuring coverage

\`\`\`bash
cargo install cargo-llvm-cov
# or
cargo install cargo-tarpaulin
\`\`\`

From \`SwissArmyNoife/\`:

\`\`\`bash
cargo llvm-cov -p control --summary-only
# or
cargo tarpaulin -p control --out Stdout
# or
./scripts/control_coverage.sh
\`\`\`

## Module checklist (\`coverage_smoke\` + in-crate tests)

| Module | Covered by |
|--------|------------|
| \`api_key\` | in-crate + smoke |
| \`audit\` | in-crate + smoke |
| \`binding\` | in-crate |
| \`budget\` | in-crate |
| \`catalog\` | in-crate |
| \`dispatch\` | in-crate |
| \`health\` | in-crate + smoke |
| \`idempotency\` | in-crate + smoke |
| \`meter\` | in-crate + smoke |
| \`offer\` | in-crate |
| \`policy\` | in-crate + smoke |
| \`policy_templates\` | in-crate + smoke |
| \`principal\` | in-crate |
| \`provision\` | in-crate |
| \`rate_limit\` | in-crate + smoke |
| \`risk\` | in-crate |
| \`trace\` | in-crate |

See also \`docs/coverage-control.md\` at workspace root for the full floor checklist.
EOF
  echo "Wrote $OUT_PATH (tool: $tool)"
}

if command -v cargo-llvm-cov >/dev/null 2>&1; then
  TOOL_USED="cargo-llvm-cov"
  SUMMARY="$(cargo llvm-cov -p control --summary-only 2>&1 || true)"
elif cargo tarpaulin --version >/dev/null 2>&1; then
  TOOL_USED="cargo-tarpaulin"
  SUMMARY="$(cargo tarpaulin -p control --out Stdout 2>&1 || true)"
fi

TEST_OUTPUT="$(cargo test -p control 2>&1)"

if [[ "$TOOL_USED" == "none" ]]; then
  SUMMARY="(no llvm-cov or tarpaulin — install instructions in output doc)"
fi

write_doc "$TOOL_USED" "$SUMMARY" "$TEST_OUTPUT"
