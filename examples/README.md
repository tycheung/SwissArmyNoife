# Example MCP client configs

These are **examples** of how a client registers the SwissArmyNoife stdio server. Adjust
`command` and `CONFIG_DIR` to your machine. Full steps: [docs/mcp-setup.md](../docs/mcp-setup.md).

| File | Notes |
|------|--------|
| [`claude_desktop_config.json`](claude_desktop_config.json) | Claude Desktop–style `mcpServers` block |

Build first:

```bash
cd SwissArmyNoife
cargo build -p mcp
# prefer release for daily use: cargo build -p mcp --release
```

- **Stdio ambient trust** — no API key for local stdio.
- Point `command` at `target/debug/mcp` or `target/release/mcp` (`.exe` on Windows).
