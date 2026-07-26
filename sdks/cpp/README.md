# C++ SDK (`sak336`)

Header/library client for SwissArmyNoife HTTP admin + MCP (cpp-httplib + nlohmann/json via
CMake FetchContent). Requires a C++17 toolchain and CMake ≥ 3.20.

## Build / test

```bash
cd SwissArmyNoife/sdks/cpp
cmake -S . -B build -G "MinGW Makefiles"
cmake --build build
ctest --test-dir build --output-on-failure
```

Scaffold only (`sak336-a`). Client surfaces land in `sak336-b` / `sak336-c`.

## Dependencies

- **Catch2** (tests) — FetchContent
- Later: **cpp-httplib**, **nlohmann/json** (HTTP/MCP) — FetchContent; TLS via OpenSSL optional
