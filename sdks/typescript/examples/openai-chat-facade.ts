/**
 * OpenAI-shaped chat facade via `SakClient.chatCompletions` (`sak546-c`).
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

import { SakClient } from "../src/index.js";

const base = process.env.SAK_HTTP ?? "http://127.0.0.1:8787";
const llmBinding = process.env.SAK_LLM_BINDING;
const toolsBinding = process.env.SAK_TOOLS_BINDING;

async function main(): Promise<void> {
  if (!llmBinding) {
    console.error("Set SAK_LLM_BINDING to an llm.chat binding UUID");
    process.exit(1);
  }

  const client = new SakClient(base);

  const chat = await client.chatCompletions({
    binding_id: llmBinding,
    model: "echo",
    messages: [{ role: "user", content: "ping" }],
  });
  console.log("chat:", chat);

  if (toolsBinding) {
    const tools = await client.chatCompletions({
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
