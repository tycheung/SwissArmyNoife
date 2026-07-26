# MCP setup (any harness)

SwissArmyNoife speaks **MCP**. Build the server, set a few env vars, then register it in
whatever client you use (Claude Desktop, Cursor, custom agents, CI). This doc covers the
server side only — wire the binary into your client however that product expects.

## Build

```bash
cd SwissArmyNoife
cargo build -p mcp
# day-to-day: cargo build -p mcp --release
```

Binaries (under `target/debug/` or `target/release/`):

| Binary | Transport |
|--------|-----------|
| `mcp` (`mcp.exe` on Windows) | **stdio** — client spawns the process and talks over stdin/stdout |
| `mcp-http` | **Streamable HTTP** — listen on a local URL |

## Stdio (default)

1. Create a config directory (broker state / SQLite live here).
2. Point the client at the `mcp` binary with env vars (see below).
3. Leave `args` empty unless your client requires placeholders.

**Auth:** stdio uses **ambient trust** — no API key. Treat it as single-user / local only.
Do not expose stdio over a network socket.

Minimal client registration shape (names vary by product):

```json
{
  "mcpServers": {
    "swissarmynoife": {
      "command": "/absolute/path/to/mcp",
      "args": [],
      "env": {
        "CONFIG_DIR": "/absolute/path/to/SwissArmyNoife/.run",
        "LLM_BACKEND": "echo"
      }
    }
  }
}
```

On Windows use forward slashes in JSON paths, or escaped backslashes. Example configs:
[`examples/claude_desktop_config.json`](../examples/claude_desktop_config.json).

Smoke without a GUI client:

```bash
cargo test -p mcp -q
# or run the binary yourself and drive MCP JSON-RPC from any stdio client
```

## Streamable HTTP (optional)

```bash
export CONFIG_DIR=/absolute/path/to/SwissArmyNoife/.run
export LLM_BACKEND=echo
export MCP_HTTP_TOKEN=dev-token   # required unless insecure loopback
cargo run -p mcp --bin mcp-http
```

| Variable | Notes |
|----------|--------|
| `MCP_HTTP_TOKEN` | Clients send `Authorization: Bearer <token>` |
| `MCP_HTTP_ALLOW_INSECURE=1` | Loopback/tests only — skips token |
| `MCP_HTTP_ADDR` | Default `127.0.0.1:8080` |

Endpoint: `http://{MCP_HTTP_ADDR}/mcp`.

## Environment

| Variable | Typical local value | Purpose |
|----------|---------------------|---------|
| `CONFIG_DIR` | `…/SwissArmyNoife/.run` | Config + DB root (create the directory once) |
| `LLM_BACKEND` | `echo` (no Ollama) or `ollama` | LLM offer backend |
| `SANDBOX_BACKEND` | `none` (default) or `stub` | Sandbox offer |
| `RUST_LOG` | `mcp=warn,rmcp=warn` | Logging (stderr) |

Full catalog: if you are in the Agentic workspace, see [`docs/env.md`](../../docs/env.md);
auth detail: [`docs/mcp-auth.md`](../../docs/mcp-auth.md).

## After rebuild

Restart or reload the MCP connection in your client so it picks up the new binary. Some
clients cache tool lists — if tools look stale, remove and re-add the server entry (or
rename the server key) and reconnect.

## Sanity tools

Once connected, try in order: `ping` → `catalog_list` → `bind` → `llm_chat` (with
`LLM_BACKEND=echo` you do not need a real model).
