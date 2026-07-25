# Registry → marketplace E2E operator hints (sak367-b)
# Prints the manual checklist commands from docs/module-registry-marketplace-e2e.md.
# Does not start servers or require live services. Always exits 0.
# Usage: from SwissArmyNoife/:  .\scripts\registry_marketplace_e2e_hint.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$AgenticRoot = Split-Path -Parent $Root
$ApiRoot = Join-Path $AgenticRoot "marketplace-api"
$ApiBase = "http://127.0.0.1:8790"

Write-Host "== registry marketplace E2E hints (sak367-b) =="
Write-Host "Doc: docs/module-registry-marketplace-e2e.md"
Write-Host ""
Write-Host "1. Start marketplace-api:"
Write-Host "  cd $ApiRoot"
Write-Host "  cargo run -p marketplace-api"
Write-Host "  curl -sf $ApiBase/health"
Write-Host ""
Write-Host "2. Publish (account + module; replace ARTIFACT / TOKEN):"
Write-Host "  curl -s -X POST $ApiBase/v1/accounts ``"
Write-Host "    -H `"Content-Type: application/json`" ``"
Write-Host "    -d '{`"display_name`":`"e2e-publisher`"}'"
Write-Host "  # POST $ApiBase/v1/modules with Authorization + artifact_base64"
Write-Host ""
Write-Host "3. Resolve:"
Write-Host "  curl -s $ApiBase/v1/modules/community.echo"
Write-Host "  curl -s $ApiBase/v1/modules/community.echo/latest"
Write-Host ""
Write-Host "4. Install (broker):"
Write-Host "  cd $Root"
Write-Host "  cargo run -p cli -- module install --registry $ApiBase community.echo"
Write-Host "  cargo run -p cli -- module list"
Write-Host ""
Write-Host "5. CLI invoke (optional):"
Write-Host "  cargo run -p cli -- module invoke community.echo 2 3"
Write-Host ""
Write-Host "6. MCP invoke:"
Write-Host "  cargo run -p mcp"
Write-Host "  # Cursor: module_list then module_invoke community.echo add {a:2,b:3}"
Write-Host ""
Write-Host "Offline CI guard (no live API):"
Write-Host "  cargo test -p module-registry registry_download_install_invoke"
Write-Host ""
Write-Host "registry_marketplace_e2e_hint: OK (hints only; no services required)"
exit 0
