# OpenAI-tools facade (`sak540` / polish `sak541–548`)

Design: Agentic [`docs/adr/011-openai-tools-facade.md`](../../docs/adr/011-openai-tools-facade.md).

Thin HTTP adapter on **http-admin** (not a new offer family). MCP remains the primary
stranger-bar surface.

## Endpoints

| Method | Path | Maps to |
|--------|------|---------|
| `POST` | `/v1/chat/completions` | `llm.chat` or `tools.loop` |

## Auth (`sak541`)

When `MCP_HTTP_TOKEN` is set (or minted `sk_live_…` API keys), every admin route —
including `/v1/chat/completions` — requires:

```http
Authorization: Bearer <token>
```

With no token configured, local process auth is off (tests / ambient demos only — not a
network trust story). `MCP_HTTP_ALLOW_INSECURE=1` documents intentional disable for local
dev; prefer setting a token for anything beyond loopback experiments.

## Chat (`llm.chat`)

1. Start admin: `cargo run -p http-admin` (default `http://127.0.0.1:8787`).
2. Create an `llm.chat` binding (MCP `bind`, or test helper / future bind HTTP).
3. Call completions with that `binding_id`:

```bash
curl -s http://127.0.0.1:8787/v1/chat/completions \
  -H 'content-type: application/json' \
  -H "Authorization: Bearer $MCP_HTTP_TOKEN" \
  -d '{
    "binding_id": "<llm.chat-binding-uuid>",
    "model": "echo",
    "messages": [{ "role": "user", "content": "ping" }]
  }'
```

Header alternative: `X-Sak-Binding-Id: <uuid>`.

Message `content` must be a **string** (or null/omitted). Multimodal arrays
(`image_url`, …) return HTTP 400 `schema.invalid` (`sak544-b`).

### Streaming (`sak542`)

`stream: true` on the **chat** path returns `Content-Type: text/event-stream` with
OpenAI-ish `chat.completion.chunk` data lines and a terminal `data: [DONE]`.

```bash
curl -N http://127.0.0.1:8787/v1/chat/completions \
  -H 'content-type: application/json' \
  -H "Authorization: Bearer $MCP_HTTP_TOKEN" \
  -d '{
    "binding_id": "<llm.chat-binding-uuid>",
    "model": "echo",
    "stream": true,
    "messages": [{ "role": "user", "content": "ping" }]
  }'
```

Offer errors after the stream starts are framed as an SSE `error` object (then `[DONE]`).
Binding miss / bad UUID before the stream starts stays JSON (HTTP 4xx).

`stream: true` with **`tool_calls`** → HTTP 400 (`stream_not_supported`).

## Tools round-trip (`tools.loop`)

When the **last** message includes OpenAI-shaped `tool_calls`, the facade invokes
`tools.loop` instead of `llm.chat`. Supply `tools_binding_id` (or
`X-Sak-Tools-Binding-Id`):

```bash
curl -s http://127.0.0.1:8787/v1/chat/completions \
  -H 'content-type: application/json' \
  -H "Authorization: Bearer $MCP_HTTP_TOKEN" \
  -d '{
    "tools_binding_id": "<tools.loop-binding-uuid>",
    "messages": [{
      "role": "assistant",
      "tool_calls": [{
        "id": "call_1",
        "type": "function",
        "function": {
          "name": "tools.echo",
          "arguments": "{\"message\":\"hi\"}"
        }
      }]
    }]
  }'
```

Response `choices[0].finish_reason` is `tool_calls`; `message.content` is the loop result
JSON (includes per-tool `ok` / `output`).

Streaming is **not** supported on this path (`stream_not_supported`, `sak543-a`).

## Errors and limits (`sak544`)

JSON errors use OpenAI-ish `{ "error": { "message", "type", "code" } }` (no secrets).

| Broker / facade signal | HTTP | `error.type` (typical) |
|------------------------|------|-------------------------|
| `schema.invalid` / facade validation | 400 | `invalid_request_error` |
| `policy.denied` / `egress.denied` | 403 | `permission_error` |
| binding miss / expired | 404 | `invalid_request_error` |
| rate limit (`SAK_RATE_LIMIT_PER_MIN` / test limiter) | 429 | `rate_limit_error` (`budget.exhausted`) |
| provider unreachable | 502 | `server_error` |

## SDK sketch

| Language | Entry |
|----------|-------|
| Rust | [`crates/sdk` example `chat_completions`](../crates/sdk/examples/chat_completions.rs) (`SakClient::chat_completions`) |
| TypeScript | [`sdks/typescript/examples/openai-chat-facade.ts`](../sdks/typescript/examples/openai-chat-facade.ts) (`SakClient.chatCompletions`) |
| Python | `SakClient.chat_completions` in [`sdks/python`](../sdks/python/src/swissarmynoife/client.py) |

OpenAPI stub: [`docs/openapi/sak-admin.v0.yaml`](openapi/sak-admin.v0.yaml) path
`/v1/chat/completions`.
