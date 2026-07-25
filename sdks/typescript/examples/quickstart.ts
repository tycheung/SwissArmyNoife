/**
 * Quickstart sketch (`sak324-b`) — run against a live `http-admin`.
 *
 * ```bash
 * cargo run -p http-admin
 * npx tsx examples/quickstart.ts
 * # or: SAK_HTTP=http://127.0.0.1:8787 npx tsx examples/quickstart.ts
 * ```
 */

import { SakClient } from "../src/index.js";

async function main(): Promise<void> {
  const base = process.env.SAK_HTTP ?? "http://127.0.0.1:8787";
  const sak = new SakClient(base);

  const health = await sak.health();
  const modules = await sak.listModules();

  console.log("health:", health);
  console.log("modules:", modules);
}

main().catch((err: unknown) => {
  console.error(err);
  process.exit(1);
});
