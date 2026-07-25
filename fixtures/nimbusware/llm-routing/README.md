# LLM routing golden fixtures (`sak142`)

Hand-authored samples for Nimbusware dual-run parity on **provider routing** (echo vs
vault-backed providers). Files use the shared `sak.fixture.nimbusware/v0` envelope; see
[`../README.md`](../README.md).

Consumed by `offer-llm/tests/golden_llm_routing.rs`.

| File | Case |
|------|------|
| `echo-chat.json` | `LLM_BACKEND=echo` single user turn via `EchoChatProvider` (`sak142-a`) |
| `echo-chat-system.json` | System preamble + user turn; echo uses last message content (`sak142-b`) |
| `vault-missing.json` | `vault.missing` when `connection_id` absent from catalog (`sak142-c`) |
| `provider-openai-hint.json` | OpenAI `provider_default` routing hint; no secrets in fixture (`sak142-c`) |
