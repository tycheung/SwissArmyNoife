# Offer behavioral golden fixtures

Hand-authored JSON used by `offer-*` crate golden tests to lock `InvokeReq` /
`InvokeResp` shapes for SwissArmyNoife offers.

## Layout

```text
fixtures/offers/
  README.md                 ← this file
  llm-routing/              ← LLM resolve + chat golden cases
    echo-chat.json
    echo-chat-system.json
    vault-missing.json      ← vault.missing when connection_id absent from catalog
    provider-openai-hint.json ← openai provider_default routing (no secrets)
  sandbox/                  ← filesystem jail / argv goldens
  <offer-id>.<case>.json    ← one scenario per file (other offers)
```

## File format (v0)

Each file is a JSON object:

| Field | Type | Meaning |
|-------|------|---------|
| `schema` | string | Always `sak.fixture.offer/v0` |
| `offer` | string | Offer id (e.g. `llm.chat`) |
| `source` | string | Provenance note (hand-authored, export path, …) |
| `request` | object | Wire-shaped `InvokeReq` (see `types`) |
| `expect` | object | Expected `InvokeResp` **or** constrained fields to assert |

Secrets must never appear: use placeholders like `"***REDACTED***"`.

## Adding fixtures

1. Hand-author minimal cases that encode the contract under test.
2. Name files `{offer}.{case}.json` using dots in the offer id as-is
   (`llm.chat.roundtrip.json`).

## Consumers

Offer crate golden tests load these via `types::load_offer_fixture` (filesystem under
`CARGO_MANIFEST_DIR`) or `include_str!`. Do not put fixtures inside `target/`.

Related: MCP *tool* step scripts live under [`../mcp/conformance/`](../mcp/conformance/)
and are separate from this tree.
