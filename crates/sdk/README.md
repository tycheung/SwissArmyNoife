# sdk — Rust HTTP admin client (`sak320`)

Thin `reqwest` wrapper over the broker **`http-admin`** API. MCP tools (llm, sandbox, memory, …)
use stdio or Streamable HTTP — this crate covers **HTTP admin only** in v0.

**MCP clients:** use the TypeScript / Python `SakMcpClient` (session `initialize` + `bind` /
`invoke`, `sak329`) or the Rust [`rmcp`](https://crates.io/crates/rmcp) crate against
`mcp-http`. A first-party Rust `SakMcpClient` is deferred (`sak329-d`).


## Quickstart

```bash
# Terminal 1 — broker admin (default 127.0.0.1:8787)
cargo run -p http-admin

# Terminal 2
cargo run -p sdk --example quickstart
# optional: SAK_HTTP=http://127.0.0.1:8787 cargo run -p sdk --example quickstart
```

Workspace doc: [`docs/sdk-quickstart.md`](../../../docs/sdk-quickstart.md) (`sak324-a`).

## `SakClient`

```rust
use sdk::SakClient;

#[tokio::main]
async fn main() -> Result<(), sdk::SdkError> {
    let client = SakClient::new("http://127.0.0.1:8787");
    let health = client.health().await?;
    let modules = client.list_modules().await?;
    let capacity = client.capacity().await?;
    let work = client.list_work().await?;
    let nodes = client.list_nodes().await?;
    // ...
    Ok(())
}
```

| Method | HTTP route | Slice |
|--------|------------|-------|
| `health()` | `GET /health` | sak320-b |
| `list_modules()` | `GET /v1/sak/modules` | sak320-b |
| `get_module(id)` | `GET /v1/sak/modules/{id}` | sak320-b |
| `capacity()` | `GET /v1/sak/capacity` | sak320-c |
| `list_work()` | `GET /v1/sak/compute/work` | sak320-c |
| `list_nodes()` | `GET /v1/sak/compute/nodes` | sak320-c |

Errors map to [`SdkError`](src/error.rs) (`Http` transport/status, `Schema` JSON parse).

## Tests

In-crate wiremock tests cover health, modules, capacity, and compute work list (`client.rs`).
