# SwissArmyNoife local CI gate (Windows)
# Usage: from SwissArmyNoife/:  .\scripts\ci_check.ps1
# Cross-repo matrix (Nimbusware, marketplace-api/web): ..\docs\ci-matrix.md
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path

Write-Host "== cargo fmt --check =="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo clippy =="
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo test =="
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== xtask boundaries =="
cargo run -q -p xtask -- boundaries
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== xtask conformance =="
cargo run -q -p xtask -- conformance
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== xtask schema export --check =="
cargo run -q -p xtask -- schema export --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo deny check licenses =="
if (-not (Get-Command cargo-deny -ErrorAction SilentlyContinue)) {
    Write-Host "cargo-deny not found; installing..."
    cargo install cargo-deny --locked
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
cargo deny check licenses
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "ci_check: OK"
