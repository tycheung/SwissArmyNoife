# offer-llm

First-party `llm.*` offers: resolve, chat, embed, preflight, stream, telemetry, Ollama manage.

## `llm.chat` args

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `messages` | array | yes | `{ role, content }` chat turns |
| `model` | string | yes | Model id for the resolved provider |
| `max_tokens` | number | no | Passed through when set |
| `temperature` | number | no | Passed through when set |
| `prompt_cache_key` | string | no | Optional provider prompt-cache hint (`sak141`); forwarded on [`ChatRequest`](../../provider-core/src/chat.rs) and ignored by backends that do not support caching |

MCP `llm_chat` accepts the same optional `prompt_cache_key` field on tool args.
