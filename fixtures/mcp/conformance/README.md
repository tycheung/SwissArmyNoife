# MCP conformance fixtures (`sak106`)

JSON step scripts exercised by the integration test harness in
`SwissArmyNoife/crates/mcp/tests/conformance_fixture.rs`.

## Fixtures

| File | Test name | Steps (summary) |
|------|-----------|-----------------|
| `ping-catalog.json` | `ping-catalog-smoke` | `ping` → `catalog_list` (expects `broker.health`, `llm.chat`, …) |
| `bind-ping-pack.json` | `bind-ping-pack` | `session_bind` pack → `ping` |
| `invoke-deny.json` | `invoke-deny` | Policy deny path on `invoke` |
| `memory-search-empty.json` | `memory-search-empty` | Bind memory → empty `memory_search` |
| `llm-chat-echo.json` | `llm-chat-echo` | Bind LLM → `llm_chat` with echo backend (`LLM_BACKEND=echo`) |

Each fixture is `{ "name", "version", "steps": [{ "tool", "arguments?", "expect_contains" }] }`.
Binding ids from prior steps can be referenced as `"$binding:<key>"` in later arguments.

## Run

From `SwissArmyNoife/` (builds `mcp` binary first):

```bash
cargo test -p mcp --test conformance_fixture
```

Run a single fixture:

```bash
cargo test -p mcp --test conformance_fixture conformance_ping_catalog_fixture
cargo test -p mcp --test conformance_fixture conformance_llm_chat_echo_fixture
```

Environment set by the harness: `CONFIG_DIR` (temp), `LLM_BACKEND=echo`, `SANDBOX_BACKEND=none`,
`CAPACITY_PROBE=fake`.

## v0 scope

This suite is the **v0 fixture pack** (`sak106` **done**). Additional fixtures for new offer
families can land in follow-up epics without reopening the skeleton harness.
