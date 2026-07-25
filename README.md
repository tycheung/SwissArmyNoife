# SwissArmyNoife

A **local-first capability broker**: install it once, then any harness (Cursor, Claude Desktop,
Nimbusware, …) can call tools like chat, sandbox, filesystem, memory, and research over MCP.

Shorthand in docs: **sak**. License: **Apache-2.0** ([LICENSE](LICENSE)).

## Quick start

```bash
cd SwissArmyNoife
cargo build -p mcp
cargo run -p cli -- hello
```

Point Cursor at the built `mcp` binary — copy [`.cursor/mcp.json.example`](../.cursor/mcp.json.example)
into the Agentic workspace `.cursor/mcp.json` and adjust paths. Full walkthrough:
[Windows first-run](../docs/windows-first-run.md) · [env vars](../docs/env.md).

Smoke checks and CI:

```bash
.\scripts\ci_check.ps1    # Windows
./scripts/ci_check.sh     # Unix
```

More: [CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md).

## What it provides

- **MCP** (stdio + HTTP): catalog, bind, invoke, plus first-party tools (`llm_chat`, `sandbox_exec`, `fs_*`, `memory_*`, …)
- **Wasm modules** first for marketplace add-ons (process secondary; no untrusted native dylibs)
- **Local-first** defaults (SQLite)

Sibling repos in this workspace: [marketplace-api](../marketplace-api),
[marketplace-web](../marketplace-web), [Nimbusware](../Nimbusware).

## Documentation

| Topic | Where |
|-------|--------|
| Doc index | [../docs/README.md](../docs/README.md) |
| First run / Cursor MCP | [../docs/windows-first-run.md](../docs/windows-first-run.md) |
| Environment | [../docs/env.md](../docs/env.md) |
| Crate boundaries / size gates | [../docs/crate-boundaries.md](../docs/crate-boundaries.md) · [../docs/crate-size-budgets.md](../docs/crate-size-budgets.md) |
| Publish dry-run | [../docs/publish-dry-run.md](../docs/publish-dry-run.md) |
| CI commands | [../docs/ci-matrix.md](../docs/ci-matrix.md) |

