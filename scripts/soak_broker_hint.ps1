# Soak-smoke broker launch hints (sak415-o / sak415-q)
# Prints commands to start http-admin and/or mcp-http for peel --require-live.
# Does not start long-lived servers. Use -TryHealth for optional curl probe.
# Usage: from SwissArmyNoife/:  .\scripts\soak_broker_hint.ps1 [-TryHealth]
param(
    [switch]$TryHealth
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ConfigDir = Join-Path $Root ".run"
$NimbuswareRoot = Join-Path (Split-Path -Parent $Root) "Nimbusware"
$HttpAdminPort = 8787
$McpHttpPort = 8080
$CommonPorts = @(8787, 8080)

Write-Host "== soak-smoke broker launch hints (sak415-o) =="
Write-Host ""
Write-Host "Build (once):"
Write-Host "  cd $Root"
Write-Host "  cargo build -p http-admin -p mcp"
Write-Host ""
Write-Host 'Terminal A - HTTP admin (BrokerClient / NIMBUSWARE_BROKER_HTTP):'
Write-Host "  cd $Root"
Write-Host "  `$env:CONFIG_DIR = '$ConfigDir'"
Write-Host "  `$env:LLM_BACKEND = 'echo'"
Write-Host "  `$env:HTTP_ADDR = '127.0.0.1:$HttpAdminPort'"
Write-Host "  cargo run -p http-admin"
Write-Host ""
Write-Host 'Terminal B - Streamable HTTP MCP (optional; NIMBUSWARE_BROKER_MCP):'
Write-Host "  cd $Root"
Write-Host "  `$env:CONFIG_DIR = '$ConfigDir'"
Write-Host "  `$env:LLM_BACKEND = 'echo'"
Write-Host "  `$env:MCP_HTTP_TOKEN = 'soak-dev-token'"
Write-Host "  `$env:MCP_HTTP_ADDR = '127.0.0.1:$McpHttpPort'"
Write-Host "  cargo run -p mcp --bin mcp-http"
Write-Host ""
Write-Host "Nimbusware env for peel_soak_smoke.py --require-live:"
Write-Host "  cd $NimbuswareRoot"
Write-Host "  `$env:PYTHONPATH = 'packages;tests'"
Write-Host "  `$env:NIMBUSWARE_BROKER_HTTP = 'http://127.0.0.1:$HttpAdminPort'"
Write-Host "  `$env:NIMBUSWARE_BROKER_MCP = 'http://127.0.0.1:$McpHttpPort/mcp'"
Write-Host "  `$env:NIMBUSWARE_BROKER_TOKEN = 'soak-dev-token'"
Write-Host "  python scripts/peel_soak_smoke.py --require-live"
Write-Host ""
Write-Host 'Live HTTP health ping (sak415-q; exit 0/1/2 - no server start):'
Write-Host "  cd $NimbuswareRoot"
Write-Host "  python scripts/peel_live_ping.py"
Write-Host ""
Write-Host 'Note: StartBrief omitted - start servers manually (more reliable on Windows).'
Write-Host "Doc: docs/peel-soak-day0.md"

function Test-HealthUrl {
    param([string]$BaseUrl)
    foreach ($path in @("/health", "/v1/sak/health")) {
        try {
            $uri = "$BaseUrl$path"
            $resp = Invoke-WebRequest -Uri $uri -TimeoutSec 3 -UseBasicParsing
            Write-Host "  OK $uri -> $($resp.StatusCode)"
            return $true
        } catch {
            Write-Host "  skip/fail $uri : $_"
        }
    }
    return $false
}

if ($TryHealth) {
    Write-Host ""
    Write-Host "== optional health probe (3s timeout; sak415-q) =="
    $httpUrl = $env:NIMBUSWARE_BROKER_HTTP
    if ($httpUrl) {
        $httpUrl = $httpUrl.TrimEnd("/")
        Test-HealthUrl -BaseUrl $httpUrl | Out-Null
    } else {
        Write-Host "  NIMBUSWARE_BROKER_HTTP unset - trying common ports 8787, 8080"
        foreach ($port in $CommonPorts) {
            if (Test-HealthUrl -BaseUrl "http://127.0.0.1:$port") { break }
        }
    }
    $pingScript = Join-Path $NimbuswareRoot "scripts/peel_live_ping.py"
    if (Test-Path $pingScript) {
        Write-Host ""
        Write-Host "  peel_live_ping.py (structured exit codes):"
        Push-Location $NimbuswareRoot
        try {
            & python scripts/peel_live_ping.py
            $code = $LASTEXITCODE
            Write-Host "  peel_live_ping exit: $code"
        } finally {
            Pop-Location
        }
    }
}

Write-Host ""
Write-Host "soak_broker_hint: OK"
