# Nimbusware behavioral golden fixtures

Captured (or hand-authored) JSON used to prove SwissArmyNoife offer parity against
Nimbusware capability-plane behavior **without** importing Nimbusware packages.

## Layout

```text
fixtures/nimbusware/
  README.md                 ← this file
  llm-routing/              ← LLM resolve + chat golden cases (sak142)
    echo-chat.json
    echo-chat-system.json
    vault-missing.json      ← vault.missing when connection_id absent from catalog
    provider-openai-hint.json ← openai provider_default routing (no secrets)
  <offer-id>.<case>.json    ← one scenario per file (other offers)
```

## File format (v0)

Each file is a JSON object:

| Field | Type | Meaning |
|-------|------|---------|
| `schema` | string | Always `sak.fixture.nimbusware/v0` |
| `offer` | string | Offer id (e.g. `llm.chat`) |
| `source` | string | Provenance note (hand-authored, export path, …) |
| `request` | object | Wire-shaped `InvokeReq` (see `types`) |
| `expect` | object | Expected `InvokeResp` **or** constrained fields to assert |

Secrets must never appear: use placeholders like `"***REDACTED***"`.

## Adding fixtures

1. Prefer real Nimbusware dual-run exports once Phase 9 exists.
2. Until then, hand-author minimal cases that encode the contract under test.
3. Name files `{offer}.{case}.json` using dots in the offer id as-is
   (`llm.chat.roundtrip.json`).

## Consumers

Future conformance / offer tests will load these via `include_str!` or filesystem under
`CARGO_MANIFEST_DIR`. Do not put fixtures inside `target/`.
