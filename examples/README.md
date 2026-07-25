# Claude Desktop — SwissArmyNoife MCP

Copy [`claude_desktop_config.json`](claude_desktop_config.json) into Claude Desktop’s
MCP config (merge under `mcpServers`), then set `command` to your built `mcp` binary.

- **Stdio ambient trust** — no API key (see `docs/mcp-auth.md` in the Agentic workspace).
- Build: from `SwissArmyNoife/`, `cargo build -p mcp`.
- Prefer release builds for day-to-day use: `cargo build -p mcp --release` and point
  `command` at `target/release/mcp` (`.exe` on Windows).
