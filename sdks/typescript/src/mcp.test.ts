import assert from "node:assert/strict";
import test from "node:test";

import { SakMcpClient } from "./mcp.js";

/** Existing tool tests skip handshake noise (`autoInitialize: false`). */
function toolClient(fetch: typeof globalThis.fetch, token?: string) {
  return new SakMcpClient("http://127.0.0.1:8080/mcp", {
    token,
    fetch,
    autoInitialize: false,
  });
}

test("initialize captures mcp-session-id and sends initialized (sak329-a)", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const mockFetch: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    const body = JSON.parse(String(init?.body ?? "{}")) as { method?: string };
    if (body.method === "initialize") {
      return new Response(
        JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          result: { protocolVersion: "2024-11-05", capabilities: {} },
        }),
        {
          status: 200,
          headers: {
            "Content-Type": "application/json",
            "mcp-session-id": "sess-abc",
          },
        },
      );
    }
    if (body.method === "notifications/initialized") {
      return new Response(null, { status: 202 });
    }
    if (body.method === "tools/call") {
      const headers = init?.headers as Record<string, string>;
      assert.equal(headers["mcp-session-id"], "sess-abc");
      return new Response(
        JSON.stringify({
          jsonrpc: "2.0",
          id: 2,
          result: { content: [{ type: "text", text: "pong" }] },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    }
    throw new Error(`unexpected method ${body.method}`);
  };

  const client = new SakMcpClient("http://127.0.0.1:8080/mcp", { fetch: mockFetch });
  await client.initialize();
  assert.equal(client.getSessionId(), "sess-abc");
  const out = await client.ping();
  assert.equal(out, "pong");
  assert.equal(calls.length, 3);
  assert.equal(JSON.parse(String(calls[0]?.init?.body)).method, "initialize");
  assert.equal(
    JSON.parse(String(calls[1]?.init?.body)).method,
    "notifications/initialized",
  );
  assert.equal(JSON.parse(String(calls[2]?.init?.body)).method, "tools/call");
});

test("ping auto-initializes once (sak329-a)", async () => {
  let initCount = 0;
  const mockFetch: typeof fetch = async (_input, init) => {
    const body = JSON.parse(String(init?.body ?? "{}")) as { method?: string };
    if (body.method === "initialize") {
      initCount += 1;
      return new Response(
        JSON.stringify({ jsonrpc: "2.0", id: 1, result: {} }),
        {
          status: 200,
          headers: {
            "Content-Type": "application/json",
            "mcp-session-id": "s1",
          },
        },
      );
    }
    if (body.method === "notifications/initialized") {
      return new Response(null, { status: 202 });
    }
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 2,
        result: { content: [{ type: "text", text: "pong" }] },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };

  const client = new SakMcpClient("http://127.0.0.1:8080/mcp", { fetch: mockFetch });
  await client.ping();
  await client.ping();
  assert.equal(initCount, 1);
  assert.equal(client.getSessionId(), "s1");
});

test("ping posts tools/call and returns text content", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const mockFetch: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: { content: [{ type: "text", text: "pong" }] },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };

  const client = toolClient(mockFetch, "tok");
  const out = await client.ping();

  assert.equal(out, "pong");
  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.url, "http://127.0.0.1:8080/mcp");
  assert.equal(calls[0]?.init?.method, "POST");
  const headers = calls[0]?.init?.headers as Record<string, string>;
  assert.equal(headers.Authorization, "Bearer tok");
  const body = JSON.parse(String(calls[0]?.init?.body));
  assert.equal(body.method, "tools/call");
  assert.equal(body.params.name, "ping");
});

test("toolsList posts tools/list", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const mockFetch: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: { tools: [{ name: "ping" }, { name: "catalog_list" }] },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };

  const client = toolClient(mockFetch);
  const out = await client.toolsList();

  assert.deepEqual(out, { tools: [{ name: "ping" }, { name: "catalog_list" }] });
  assert.equal(calls.length, 1);
  const body = JSON.parse(String(calls[0]?.init?.body));
  assert.equal(body.method, "tools/list");
});

test("catalogList posts tools/call catalog_list", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const mockFetch: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: { offers: [{ id: "llm.chat" }] },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };

  const client = toolClient(mockFetch);
  const out = await client.catalogList();

  assert.deepEqual(out, { offers: [{ id: "llm.chat" }] });
  assert.equal(calls.length, 1);
  const body = JSON.parse(String(calls[0]?.init?.body));
  assert.equal(body.method, "tools/call");
  assert.equal(body.params.name, "catalog_list");
});

test("computeWork posts tools/call compute_work (sak489-i)", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const mockFetch: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                status: "ok",
                result: { action: "enqueue", work: { id: "w1" } },
              }),
            },
          ],
        },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };

  const client = toolClient(mockFetch);
  const out = await client.computeWork({
    binding_id: "b1",
    action: "enqueue",
    kind: "echo",
  });

  assert.equal((out as { action?: string }).action, "enqueue");
  const body = JSON.parse(String(calls[0]?.init?.body));
  assert.equal(body.method, "tools/call");
  assert.equal(body.params.name, "compute_work");
  assert.equal(body.params.arguments.binding_id, "b1");
});

test("claimWork normalize empty vs broker_miss (sak489-i / sak490-i)", async () => {
  const { SakClient } = await import("./index.js");
  const empty = SakClient.normalizeClaimWorkResponse({
    work: null,
    error: "queue empty",
  });
  assert.deepEqual(empty, { work: null, via: "broker" });
  assert.throws(
    () =>
      SakClient.normalizeClaimWorkResponse({
        via: "broker_miss",
        work: null,
        error: "queue empty",
      }),
    /broker_miss/,
  );
});

test("requeueWork posts action requeue (sak430-h)", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const orig = globalThis.fetch;
  globalThis.fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        action: "requeue",
        work: { id: "w1", status: "queued" },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };
  try {
    const { SakClient } = await import("./index.js");
    const client = new SakClient("http://127.0.0.1:8787");
    const out = await client.requeueWork("w1");
    assert.equal((out as { action?: string }).action, "requeue");
    assert.equal(calls.length, 1);
    assert.equal(calls[0]?.url, "http://127.0.0.1:8787/v1/sak/compute/work");
    const body = JSON.parse(String(calls[0]?.init?.body));
    assert.equal(body.action, "requeue");
    assert.equal(body.work_id, "w1");
  } finally {
    globalThis.fetch = orig;
  }
});

test("enqueueWork posts action enqueue (sak431-h)", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const orig = globalThis.fetch;
  globalThis.fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        action: "enqueue",
        work: { id: "w2", status: "queued" },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };
  try {
    const { SakClient } = await import("./index.js");
    const client = new SakClient("http://127.0.0.1:8787");
    const out = await client.enqueueWork("echo", { n: 1 });
    assert.equal((out as { action?: string }).action, "enqueue");
    const body = JSON.parse(String(calls[0]?.init?.body));
    assert.equal(body.action, "enqueue");
    assert.equal(body.kind, "echo");
  } finally {
    globalThis.fetch = orig;
  }
});

test("sak498-g mcp domain helpers raise on peel miss", async () => {
  const client = toolClient(async () =>
    new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: { via: "broker_miss", feature: "sandbox_exec", error: "down" },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    ),
  );
  await assert.rejects(() => client.sandboxExec(["echo"]), /broker_miss/);
});

test("memoryIndex posts tools/call memory_index (sak499-i)", async () => {
  const calls: { url: string; init?: RequestInit }[] = [];
  const mockFetch: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: {
          rebuilt: true,
          vector_count: 2,
          fingerprint: "fp1",
          backend: "exact",
          scope_key: "default",
        },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  };

  const client = toolClient(mockFetch);
  const out = await client.memoryIndex("b-mem", [
    { id: "1", text: "alpha" },
    { id: "2", text: "beta" },
  ]);

  assert.equal((out as { rebuilt?: boolean }).rebuilt, true);
  assert.equal(calls.length, 1);
  const body = JSON.parse(String(calls[0]?.init?.body));
  assert.equal(body.method, "tools/call");
  assert.equal(body.params.name, "memory_index");
  assert.equal(body.params.arguments.binding_id, "b-mem");
  assert.equal(body.params.arguments.documents.length, 2);
});

test("sak499-i memory peel envelope on SakMcpClient", () => {
  assert.equal(SakMcpClient.isMemoryMiss({ code: "broker_memory_only" }), true);
  assert.equal(
    SakMcpClient.isMemoryMiss({ hits: [], via: "broker" }),
    false,
  );
  assert.deepEqual(SakMcpClient.assertMemoryOk({ hits: [] }), { hits: [] });
  assert.throws(
    () =>
      SakMcpClient.assertMemoryOk({
        via: "broker_miss",
        feature: "fleet_memory_search",
        error: "down",
        hits: [],
      }),
    /broker_miss/,
  );
});

test("sak499-i memoryIndex raises on peel miss", async () => {
  const client = toolClient(async () =>
    new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: {
          code: "broker_memory_only",
          error: "use SwissArmyNoife memory_index",
        },
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    ),
  );
  await assert.rejects(
    () => client.memoryIndex("b1", [{ id: "1", text: "x" }]),
    /broker_miss/,
  );
});
