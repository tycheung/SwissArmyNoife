# SwissArmyNoife publish dry-run helper (`sak326-b`)
# Usage: from SwissArmyNoife/:  .\scripts\publish_dry_run.ps1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path

Write-Host "== cargo publish dry-run (Rust crates) =="
$cargoCrates = @("types", "sdk", "module-manifest", "module-registry")
foreach ($crate in $cargoCrates) {
    Write-Host "-- cargo publish -p $crate --dry-run --"
    cargo publish -p $crate --dry-run
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host ""
Write-Host "== npm dry-run (TypeScript SDK) =="
Write-Host "cd $Root\sdks\typescript"
Write-Host "npm ci"
Write-Host "npm run build"
Write-Host "npm publish --dry-run"
Write-Host "(run the above manually or in CI before release)"

Write-Host ""
Write-Host "== PyPI build (Python SDK) =="
Write-Host "cd $Root\sdks\python"
Write-Host "python -m pip install --upgrade build"
Write-Host "python -m build"
Write-Host "(inspect dist/; twine upload only after sign-off)"

Write-Host ""
Write-Host "publish_dry_run: OK (Rust dry-runs passed; see docs/publish-dry-run.md for full checklist)"
