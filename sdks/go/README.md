# Go SDK (`sak330`)

HTTP admin client (`Client`) and Streamable HTTP MCP client (`McpClient`) for SwissArmyNoife.

## Install / test

```bash
cd SwissArmyNoife/sdks/go
go test ./...
```

Module path: `github.com/tycheung/swissarmynoife-sdk` (Go 1.22+).

## Usage

```go
package main

import (
	"fmt"
	"log"

	sak "github.com/tycheung/swissarmynoife-sdk"
)

func main() {
	c := sak.NewClient("http://127.0.0.1:8787")
	health, err := c.Health()
	if err != nil {
		log.Fatal(err)
	}
	modules, _ := c.ListModules()
	capacity, _ := c.Capacity()
	fmt.Println(health, modules, capacity)
}
```

### MCP (`McpClient`)

On first tool call (or explicit `Initialize()`), negotiates a session and sends
`mcp-session-id` on later RPCs.

| Method | MCP wire |
|--------|----------|
| `Initialize()` | `initialize` + `notifications/initialized` |
| `Ping()` | `tools/call` → `ping` |
| `ToolsList()` | `tools/list` |
| `CatalogList()` | `tools/call` → `catalog_list` |

```go
m := sak.NewMcpClient("http://127.0.0.1:8080/mcp")
m.Token = os.Getenv("MCP_HTTP_TOKEN")
pong, err := m.Ping()
```

Set `AutoInitialize = false` in unit tests that mock a single RPC.

## Examples

See [`examples/quickstart.go`](examples/quickstart.go):

```bash
cargo run -p http-admin
cd SwissArmyNoife/sdks/go
go run ./examples/quickstart.go
```

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
