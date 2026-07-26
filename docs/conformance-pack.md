# Harness conformance pack (`sak529`)

One command runs the **MCP fixture pack** plus **offer golden** integration tests.
Use this before claiming a broker surface change is harness-ready.

## Run locally

From `SwissArmyNoife/`:

```bash
cargo run -q -p xtask -- conformance
```

Exit `0` means all packs passed. Non-zero lists which `cargo test -p … --test …` failed.

### Pack contents

| Package | Integration test |
|---------|------------------|
| `mcp` | `conformance_fixture` |
| `offer-llm` | `golden_llm_routing` |
| `offer-sandbox` | `golden_sandbox` |
| `offer-memory` | `golden_memory` |
| `offer-egress` | `golden_egress` |
| `types` | `offer_fixtures` |

Fixture details: [`fixtures/mcp/conformance/README.md`](../fixtures/mcp/conformance/README.md).
Offer goldens: [`fixtures/offers/README.md`](../fixtures/offers/README.md).

## CI

The broker GitHub Actions `check` job runs the same command after `xtask boundaries`
(`sak529-b`).

## Windows note

If `cargo test -p mcp` fails with **Access is denied** on `target/debug/mcp.exe`, stop the
Cursor (or other) MCP process that holds the lock, then re-run.

## Single fixture

```bash
cargo test -p mcp --test conformance_fixture conformance_ping_catalog_fixture
```
