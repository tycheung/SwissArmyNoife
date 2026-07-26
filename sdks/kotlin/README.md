# Kotlin SDK (`sak334`)

**Decision (sak334-a):** idiomatic Kotlin JVM client (Gradle Kotlin DSL), not Java-JAR
interop-only. Java consumers can still use `sdks/java`.

HTTP admin (`SakClient`) and Streamable HTTP MCP (`SakMcpClient`) for SwissArmyNoife.

## Build / test

Requires JDK 17+.

```bash
cd SwissArmyNoife/sdks/kotlin
gradle test
# or: ./gradlew test once the wrapper is checked in
```

## Usage

```kotlin
import com.swissarmynoife.sdk.SakClient

val sak = SakClient("http://127.0.0.1:8787")
val health = sak.health()
val modules = sak.listModules()
```

### MCP

```kotlin
import com.swissarmynoife.sdk.SakMcpClient

val mcp = SakMcpClient("http://127.0.0.1:8080/mcp")
mcp.token = System.getenv("MCP_HTTP_TOKEN")
val pong = mcp.ping()
```

Set `autoInitialize = false` in unit tests that mock a single RPC.

## Examples

See [`examples/Quickstart.kt`](examples/Quickstart.kt).

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
