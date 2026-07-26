# Java SDK (`sak331`)

HTTP admin client (`SakClient`) and Streamable HTTP MCP client (`SakMcpClient`) for SwissArmyNoife.

## Build / test

Requires JDK 17+.

```bash
cd SwissArmyNoife/sdks/java
mvn test
```

Artifact: `com.swissarmynoife:swissarmynoife-sdk:0.1.0-SNAPSHOT`

## Usage

```java
import com.swissarmynoife.sdk.SakClient;

var sak = new SakClient("http://127.0.0.1:8787");
var health = sak.health();
var modules = sak.listModules();
```

### MCP

```java
import com.swissarmynoife.sdk.SakMcpClient;

var mcp = new SakMcpClient("http://127.0.0.1:8080/mcp");
mcp.setToken(System.getenv("MCP_HTTP_TOKEN"));
String pong = mcp.ping();
```

Set `setAutoInitialize(false)` in unit tests that mock a single RPC.

## Examples

See [`examples/Quickstart.java`](examples/Quickstart.java) (compile against the jar after `mvn package`).

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
