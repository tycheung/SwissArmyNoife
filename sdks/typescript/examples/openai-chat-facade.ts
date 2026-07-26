/**
 * OpenAI-shaped chat facade sketch (`sak540-d`).
 *
 * Requires a running `http-admin` and pre-created bindings (e.g. via MCP `bind`):
 * - `SAK_LLM_BINDING` — `llm.chat` binding UUID
 * - optional `SAK_TOOLS_BINDING` — `tools.loop` binding UUID for tool_calls path
 *
 * ```bash
 * cargo run -p http-admin
 * SAK_LLM_BINDING=<uuid> npx tsx examples/openai-chat-facade.ts
 * ```
 */

const base = process.env.SAK_HTTP ?? "http://127.0.0.1:8787";
const llmBinding = process.env.SAK_LLM_BINDING;
const toolsBinding = process.env.SAK_TOOLS_BINDING;

async function chatCompletions(body: Record<string, unknown>): Promise<unknown> {
  const res = await fetch(`${base}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  const json: unknown = await res.json();
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${JSON.stringify(json)}`);
  }
  return json;
}

async function main(): Promise<void> {
  if (!llmBinding) {
    console.error("Set SAK_LLM_BINDING to an llm.chat binding UUID");
    process.exit(1);
  }

  const chat = await chatCompletions({
    binding_id: llmBinding,
    model: "echo",
    messages: [{ role: "user", content: "ping" }],
  });
  console.log("chat:", chat);

  if (toolsBinding) {
    const tools = await chatCompletions({
      tools_binding_id: toolsBinding,
      messages: [
        {
          role: "assistant",
          tool_calls: [
            {
              id: "call_1",
              type: "function",
              function: {
                name: "tools.echo",
                arguments: JSON.stringify({ message: "hi" }),
              },
            },
          ],
        },
      ],
    });
    console.log("tools:", tools);
  }
}

main().catch((err: unknown) => {
  console.error(err);
  process.exit(1);
});
