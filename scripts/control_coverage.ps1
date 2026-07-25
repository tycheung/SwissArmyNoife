# Control crate coverage measurement (sak063-d).
# Prefers cargo-llvm-cov, then cargo-tarpaulin, else runs tests + writes checklist stub.

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Invoke-Cargo {
    param([string[]]$CargoArgs)
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $out = & cargo @CargoArgs 2>&1 | Out-String
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    if ($code -ne 0) {
        throw "cargo $($CargoArgs -join ' ') failed with exit code $code`n$out"
    }
    return $out.Trim()
}

$OutPath = Join-Path $Root "docs\control-coverage.md"
$OutDir = Split-Path -Parent $OutPath
if (-not (Test-Path $OutDir)) {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
}
$Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$ToolUsed = "none"
$Summary = ""

function Get-FloorNote {
    param([string]$Tool, [string]$Body)
    if ($Tool -eq "none") {
        return "**sak063 status: partial** - coverage tool not available; module checklist from smoke tests only (>=85% floor not verified)."
    }
    if ($Body -match "(\d+\.\d+)%") {
        $pct = [double]$Matches[1]
        if ($pct -ge 85) {
            return "**sak063 status: measured** - line coverage >=85% on control crate."
        }
    }
    return "**sak063 status: partial** - coverage measured but >=85% floor not confirmed from summary below."
}

$llvmCov = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
if ($llvmCov) {
    $ToolUsed = "cargo-llvm-cov"
    $Summary = Invoke-Cargo -CargoArgs @("llvm-cov", "-p", "control", "--summary-only")
} else {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    & cargo tarpaulin --version 2>&1 | Out-Null
    $tarpOk = $LASTEXITCODE -eq 0
    $ErrorActionPreference = $prevEap
    if ($tarpOk) {
        $ToolUsed = "cargo-tarpaulin"
        $Summary = Invoke-Cargo -CargoArgs @("tarpaulin", "-p", "control", "--out", "Stdout")
    }
}

$TestOutput = Invoke-Cargo -CargoArgs @("test", "-p", "control")

if ($ToolUsed -eq "none") {
    $Summary = "(no llvm-cov or tarpaulin - install instructions in output doc)"
}

$FloorNote = Get-FloorNote -Tool $ToolUsed -Body $Summary

$lines = @(
    "# Control crate coverage report (sak063-d)",
    "",
    "Generated: $Timestamp",
    "",
    "Target: **>=85% line coverage** on the ``control`` crate.",
    "",
    $FloorNote,
    "",
    "## Tool",
    "",
    "- Used: ``$ToolUsed``",
    "- Command: see Measuring coverage below",
    "",
    "## Summary",
    "",
    "``````",
    $Summary,
    "``````",
    "",
    "## Test run (fallback)",
    "",
    "``````",
    $TestOutput,
    "``````",
    "",
    "## Measuring coverage",
    "",
    "Install one of:",
    "",
    "    cargo install cargo-llvm-cov",
    "    cargo install cargo-tarpaulin",
    "",
    "From SwissArmyNoife/:",
    "",
    "    cargo llvm-cov -p control --summary-only",
    "    cargo tarpaulin -p control --out Stdout",
    "    .\scripts\control_coverage.ps1",
    "",
    "## Module checklist (coverage_smoke + in-crate tests)",
    "",
    "| Module | Covered by |",
    "|--------|------------|",
    "| api_key | in-crate + smoke |",
    "| audit | in-crate + smoke |",
    "| binding | in-crate |",
    "| budget | in-crate |",
    "| catalog | in-crate |",
    "| dispatch | in-crate |",
    "| health | in-crate + smoke |",
    "| idempotency | in-crate + smoke |",
    "| meter | in-crate + smoke |",
    "| offer | in-crate |",
    "| policy | in-crate + smoke |",
    "| policy_templates | in-crate + smoke |",
    "| principal | in-crate |",
    "| provision | in-crate |",
    "| rate_limit | in-crate + smoke |",
    "| risk | in-crate |",
    "| trace | in-crate |",
    "",
    "See also docs/coverage-control.md at workspace root for the full floor checklist."
)

Set-Content -Path $OutPath -Value ($lines -join "`n") -Encoding utf8
Write-Host "Wrote $OutPath (tool: $ToolUsed)"
