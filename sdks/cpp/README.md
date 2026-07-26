# C++ SDK (`sak336`)

C++17 library for SwissArmyNoife HTTP admin (`SakClient`) and Streamable HTTP MCP
(`SakMcpClient`). Uses **cpp-httplib** + **nlohmann/json** via CMake FetchContent.
Catch2 for tests. TLS/OpenSSL not enabled by default (HTTP local broker).

## Build / test

Requires CMake ≥ 3.20 and a C++17 compiler (MinGW/MSVC/Clang).

```bash
cd SwissArmyNoife/sdks/cpp
cmake -S . -B build -G "MinGW Makefiles"   # or your generator
cmake --build build
ctest --test-dir build --output-on-failure
```

## Usage

```cpp
#include "swissarmynoife/sak_client.hpp"

swissarmynoife::SakClient sak("http://127.0.0.1:8787");
auto health = sak.health();
auto modules = sak.list_modules();
```

### MCP

```cpp
#include "swissarmynoife/sak_mcp_client.hpp"

swissarmynoife::SakMcpClient mcp("http://127.0.0.1:8080/mcp");
mcp.set_token(std::getenv("MCP_HTTP_TOKEN") ? std::getenv("MCP_HTTP_TOKEN") : "");
auto pong = mcp.ping();
```

Set `set_auto_initialize(false)` in unit tests that mock a single RPC.

## Examples

See [`examples/quickstart.cpp`](examples/quickstart.cpp).

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
