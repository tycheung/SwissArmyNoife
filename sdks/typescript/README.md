# TypeScript SDK (`sak321`)

HTTP admin client (`SakClient`) and Streamable HTTP MCP stub (`SakMcpClient`) for SwissArmyNoife.

## Install / build

```bash
cd SwissArmyNoife/sdks/typescript
npm install
npm run build
```

## Usage

```ts
import {
  SakClient,
  type CapacityResponse,
  type ModuleListResponse,
} from "@swissarmynoife/sdk";

const sak = new SakClient("http://127.0.0.1:8787");

const health = await sak.health();
const modules: ModuleListResponse = await sak.listModules();
const capacity: CapacityResponse = await sak.capacity();
const work = await sak.listWork();
const nodes = await sak.listNodes();

console.log({ health, modules, capacity, work, nodes });
```

### MCP (`SakMcpClient`)

Streamable HTTP MCP client. On first tool call (or explicit `initialize()`), negotiates a
session (`initialize` + `notifications/initialized`) and sends `mcp-session-id` on later RPCs
(`sak329-a`).

| Method | MCP wire | Description |
|--------|----------|-------------|
| `initialize()` | `initialize` + `notifications/initialized` | Session handshake; captures `mcp-session-id` |
| `ping()` | `tools/call` → `ping` | Health smoke; returns text from tool result |
| `toolsList()` | `tools/list` | Lists broker MCP tools |
| `catalogList()` | `tools/call` → `catalog_list` | Lists catalog offers via MCP tool |

```ts
import { SakMcpClient } from "@swissarmynoife/sdk";

const mcp = new SakMcpClient("http://127.0.0.1:8080/mcp", { token: process.env.MCP_HTTP_TOKEN });
const pong = await mcp.ping();
const tools = await mcp.toolsList();
const catalog = await mcp.catalogList();
```

Pass a custom `fetch` in options for unit tests (no live MCP server required).

### List helpers

| Method | Path | Return type alias |
|--------|------|-------------------|
| `listModules()` | `/v1/sak/modules` | `ModuleListResponse` |
| `listWork()` | `/v1/sak/compute/work` | `WorkListResponse` |
| `listNodes()` | `/v1/sak/compute/nodes` | `NodeListResponse` |
| `capacity()` | `/v1/sak/capacity` | `CapacityResponse` |

Response bodies are typed as `JsonValue` until OpenAPI codegen lands (`sak323`).

## Examples

See [`examples/quickstart.ts`](examples/quickstart.ts) for a minimal health + `listModules` sketch
against a running `http-admin`:

```bash
cargo run -p http-admin
cd SwissArmyNoife/sdks/typescript
npx tsx examples/quickstart.ts
```

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).  
Marketplace registry HTTP: [`../../docs/marketplace-quickstart.md`](../../docs/marketplace-quickstart.md).

MCP session negotiation (`initialize`) lands in a later slice; `ping` posts `tools/call` directly.
