# SwissArmyNoife

A **local-first capability broker**: install it once, then any MCP-capable harness can call
tools like chat, sandbox, filesystem, memory, and research.

Shorthand in docs: **sak**. License: **Apache-2.0** ([LICENSE](LICENSE)).

## Quick start

```bash
cd SwissArmyNoife
cargo build -p mcp
cargo run -p cli -- hello
```

Start / register the MCP server with your client:

→ **[docs/mcp-setup.md](docs/mcp-setup.md)** — build, env, stdio vs HTTP (harness-agnostic).

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

## Documentation

| Topic | Where |
|-------|--------|
| MCP setup (any client) | [docs/mcp-setup.md](docs/mcp-setup.md) |
| Harness conformance pack | [docs/conformance-pack.md](docs/conformance-pack.md) |
| OpenAI chat/tools facade | [docs/openai-tools-facade.md](docs/openai-tools-facade.md) |
| Example client configs | [examples/](examples/) |
| Control coverage | [docs/control-coverage.md](docs/control-coverage.md) |

When developing inside the broader Agentic workspace, also see sibling docs for env vars,
CI matrix, and marketplace: `../docs/` (not published with this repo alone).
