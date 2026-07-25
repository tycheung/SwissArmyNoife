#!/usr/bin/env bash
# Soak-smoke broker launch hints (sak415-o / sak415-q)
# Prints commands to start http-admin and/or mcp-http for peel --require-live.
# Does not start long-lived servers. Use --try-health for optional curl probe.
# Usage: from SwissArmyNoife/:  ./scripts/soak_broker_hint.sh [--try-health]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NIMBUSWARE_ROOT="$(cd "${ROOT}/../Nimbusware" && pwd)"
CONFIG_DIR="${ROOT}/.run"
HTTP_ADMIN_PORT=8787
MCP_HTTP_PORT=8080
TRY_HEALTH=0
if [[ "${1:-}" == "--try-health" ]]; then
  TRY_HEALTH=1
fi

echo "== soak-smoke broker launch hints (sak415-o) =="
echo ""
echo "Build (once):"
echo "  cd ${ROOT}"
echo "  cargo build -p http-admin -p mcp"
echo ""
echo "Terminal A — HTTP admin (BrokerClient / NIMBUSWARE_BROKER_HTTP):"
echo "  cd ${ROOT}"
echo "  export CONFIG_DIR='${CONFIG_DIR}'"
echo "  export LLM_BACKEND=echo"
echo "  export HTTP_ADDR=127.0.0.1:${HTTP_ADMIN_PORT}"
echo "  cargo run -p http-admin"
echo ""
echo "Terminal B — Streamable HTTP MCP (optional; NIMBUSWARE_BROKER_MCP):"
echo "  cd ${ROOT}"
echo "  export CONFIG_DIR='${CONFIG_DIR}'"
echo "  export LLM_BACKEND=echo"
echo "  export MCP_HTTP_TOKEN=soak-dev-token"
echo "  export MCP_HTTP_ADDR=127.0.0.1:${MCP_HTTP_PORT}"
echo "  cargo run -p mcp --bin mcp-http"
echo ""
echo "Nimbusware env for peel_soak_smoke.py --require-live:"
echo "  cd ${NIMBUSWARE_ROOT}"
echo "  export PYTHONPATH=packages:tests"
echo "  export NIMBUSWARE_BROKER_HTTP=http://127.0.0.1:${HTTP_ADMIN_PORT}"
echo "  export NIMBUSWARE_BROKER_MCP=http://127.0.0.1:${MCP_HTTP_PORT}/mcp"
echo "  export NIMBUSWARE_BROKER_TOKEN=soak-dev-token"
echo "  python scripts/peel_soak_smoke.py --require-live"
echo ""
echo "Live HTTP health ping (sak415-q; exit 0/1/2 — no server start):"
echo "  cd ${NIMBUSWARE_ROOT}"
echo "  python scripts/peel_live_ping.py"
echo ""
echo "Note: StartBrief (auto cargo run) omitted — manual start above is more reliable."
echo "Doc: docs/peel-soak-day0.md · calendar: docs/peel-soak-calendar.md"

probe_health() {
  local base="$1"
  for path in /health /v1/sak/health; do
    if curl -sf --max-time 3 "${base}${path}" >/dev/null 2>&1; then
      echo "  OK ${base}${path}"
      return 0
    else
      echo "  skip/fail ${base}${path}"
    fi
  done
  return 1
}

if [[ "${TRY_HEALTH}" -eq 1 ]]; then
  echo ""
  echo "== optional health probe (3s timeout; sak415-q) =="
  if [[ -n "${NIMBUSWARE_BROKER_HTTP:-}" ]]; then
    HTTP_URL="${NIMBUSWARE_BROKER_HTTP%/}"
    probe_health "${HTTP_URL}" || true
  else
    echo "  NIMBUSWARE_BROKER_HTTP unset — trying common ports 8787, 8080"
    for port in 8787 8080; do
      if probe_health "http://127.0.0.1:${port}"; then
        break
      fi
    done
  fi
  if [[ -f "${NIMBUSWARE_ROOT}/scripts/peel_live_ping.py" ]]; then
    echo ""
    echo "  peel_live_ping.py (same probe, structured exit codes):"
    (cd "${NIMBUSWARE_ROOT}" && python scripts/peel_live_ping.py) || true
    echo "  peel_live_ping exit: $? (0=ok, 1=fail, 2=skipped)"
  fi
fi

echo ""
echo "soak_broker_hint: OK (hints only; start servers manually)"
