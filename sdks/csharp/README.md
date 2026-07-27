# C# SDK (`sak332`)

.NET 8 library `SwissArmyNoife.Sdk` — HTTP admin (`SakClient`) and Streamable HTTP MCP
(`SakMcpClient`) for SwissArmyNoife.

## Build / test

```bash
cd SwissArmyNoife/sdks/csharp
dotnet test
```

## Usage

```csharp
using SwissArmyNoife.Sdk;

var sak = new SakClient("http://127.0.0.1:8787");
var health = await sak.HealthAsync();
var modules = await sak.ListModulesAsync();
```

### MCP

```csharp
var mcp = new SakMcpClient("http://127.0.0.1:8080/mcp")
{
    Token = Environment.GetEnvironmentVariable("MCP_HTTP_TOKEN"),
};
var pong = await mcp.PingAsync();
```

Set `AutoInitialize = false` in unit tests that mock a single RPC.

## Examples

See [`examples/Quickstart/Program.cs`](examples/Quickstart/Program.cs):

```bash
cargo run -p http-admin
cd SwissArmyNoife/sdks/csharp/examples/Quickstart
dotnet run
```

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
