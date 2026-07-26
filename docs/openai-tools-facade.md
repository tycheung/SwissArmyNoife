# OpenAI-tools facade (`sak540`)

Design: Agentic [`docs/adr/011-openai-tools-facade.md`](../../docs/adr/011-openai-tools-facade.md).

Thin HTTP adapter on **http-admin** (not a new offer family). MCP remains the primary
stranger-bar surface.

## Endpoints

| Method | Path | Maps to |
|--------|------|---------|
| `POST` | `/v1/chat/completions` | `llm.chat` or `tools.loop` |

Auth matches other HTTP admin routes (local process; no ambient network trust story beyond
whatever you put in front of `http-admin`).

## Chat (`llm.chat`)

1. Start admin: `cargo run -p http-admin` (default `http://127.0.0.1:8787`).
2. Create an `llm.chat` binding (MCP `bind`, or test helper / future bind HTTP).
3. Call completions with that `binding_id`:

```bash
curl -s http://127.0.0.1:8787/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{
    "binding_id": "<llm.chat-binding-uuid>",
    "model": "echo",
    "messages": [{ "role": "user", "content": "ping" }]
  }'
```

Header alternative: `X-Sak-Binding-Id: <uuid>`.

`stream: true` → HTTP 400 (`stream_not_supported`) in v0.

## Tools round-trip (`tools.loop`)

When the **last** message includes OpenAI-shaped `tool_calls`, the facade invokes
`tools.loop` instead of `llm.chat`. Supply `tools_binding_id` (or
`X-Sak-Tools-Binding-Id`):

```bash
curl -s http://127.0.0.1:8787/v1/chat/completions \
  -H 'content-type: application/json' \
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

## SDK sketch

TypeScript: [`sdks/typescript/examples/openai-chat-facade.ts`](../sdks/typescript/examples/openai-chat-facade.ts).

OpenAPI stub: [`docs/openapi/sak-admin.v0.yaml`](openapi/sak-admin.v0.yaml) path
`/v1/chat/completions`.
