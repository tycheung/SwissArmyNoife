import assert from "node:assert/strict";
import test from "node:test";

import { SakClient } from "./index.js";

function mockFetch(responses: Record<string, JsonValue>[]): typeof fetch {
  let i = 0;
  return async (_input, init) => {
    void JSON.parse(String(init?.body ?? "{}"));
    const payload = responses[i++] ?? {};
    return new Response(JSON.stringify(payload), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
}

type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

test("listWorkFiltered empty vs miss (sak490-i)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: [], action: "list" },
    { work: null, action: "list" },
    { via: "broker_miss", status: "degraded", feature: "list", work: [] },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const empty = await client.listWorkFiltered({ status: "queued" });
    assert.deepEqual((empty as { work?: unknown }).work, []);
    await assert.rejects(
      () => client.listWorkFiltered({ status: "queued" }),
      /missing or non-list key work/,
    );
    await assert.rejects(
      () => client.listWorkFiltered({ status: "queued" }),
      /broker_miss/,
    );
  } finally {
    globalThis.fetch = orig;
  }
});

test("listWorkFiltered session_id empty vs miss (sak494-i)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: [], action: "list" },
    { error: "work down", work: [] },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const empty = await client.listWorkFiltered({ status: "queued", session_id: "s1" });
    assert.deepEqual((empty as { work?: unknown }).work, []);
    await assert.rejects(
      () => client.listWorkFiltered({ status: "queued", session_id: "s1" }),
      /broker_miss/,
    );
  } finally {
    globalThis.fetch = orig;
  }
});

test("sessionComputeStatus nodes ok queue fail degraded (sak494-i)", async () => {
  const nid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { nodes: [{ id: nid, label: "n1", caps: [] }], action: "list" },
    { error: "work down", work: [] },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const out = (await client.sessionComputeStatus("s1", {
      feature: "fleet_mesh",
    })) as Record<string, JsonValue>;
    assert.equal(out.via, "broker_miss");
    assert.equal(out.status, "degraded");
    assert.equal(out.queue_depth, 0);
    assert.equal((out.nodes as JsonValue[])?.length, 1);
    assert.equal(
      (out.nodes as { node_id?: string }[])?.[0]?.node_id,
      nid,
    );
    assert.match(String(out.error ?? ""), /work down/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("getWork record vs miss (sak490-i)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: { id: "w1", status: "queued" } },
    { work: null },
    { via: "broker_miss", status: "degraded", feature: "get" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.getWork("w1");
    assert.equal((ok as { work?: { id?: string } }).work?.id, "w1");
    await assert.rejects(() => client.getWork("w1"), /missing work record/);
    await assert.rejects(() => client.getWork("w1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("requeueWork success vs miss (sak490-i)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: { id: "w1", status: "queued" }, action: "requeue" },
    { via: "broker_miss", status: "degraded", feature: "requeue" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.requeueWork("w1");
    assert.equal((ok as { action?: string }).action, "requeue");
    await assert.rejects(() => client.requeueWork("w1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("assertRecordOk write path empty vs miss (sak492-h)", () => {
  assert.deepEqual(SakClient.assertRecordOk({ work: { id: "w1" } }, "work"), {
    work: { id: "w1" },
  });
  assert.throws(
    () => SakClient.assertRecordOk({ work: null }, "work"),
    /missing work record/,
  );
  assert.throws(
    () => SakClient.assertRecordOk({ work: null, error: "queue empty" }, "work"),
    /broker_miss/,
  );
  assert.throws(
    () =>
      SakClient.assertRecordOk(
        { via: "broker_miss", status: "degraded", feature: "enqueue" },
        "work",
      ),
    /broker_miss/,
  );
  assert.throws(
    () =>
      SakClient.assertRecordOk(
        { via: "broker_miss", status: "degraded", feature: "complete" },
        "work",
      ),
    /broker_miss/,
  );
});

test("enqueueWork record vs miss (sak492-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: { id: "w1", status: "queued" }, action: "enqueue" },
    { work: null, action: "enqueue" },
    { via: "broker_miss", status: "degraded", feature: "enqueue" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.enqueueWork("echo", { n: 1 });
    assert.equal((ok as { work?: { id?: string } }).work?.id, "w1");
    await assert.rejects(() => client.enqueueWork("echo", { n: 1 }), /missing work record/);
    await assert.rejects(() => client.enqueueWork("echo", { n: 1 }), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("completeWork record vs miss (sak492-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: { id: "w1", status: "done" }, action: "complete" },
    { work: null, action: "complete" },
    { via: "broker_miss", status: "degraded", feature: "complete" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.completeWork("w1", "n1");
    assert.equal((ok as { action?: string }).action, "complete");
    await assert.rejects(() => client.completeWork("w1", "n1"), /missing work record/);
    await assert.rejects(() => client.completeWork("w1", "n1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("terminateRestartWork success vs miss (sak492-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: { id: "w1", status: "queued" }, action: "requeue" },
    { via: "broker_miss", status: "degraded", feature: "requeue" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.terminateRestartWork("w1");
    assert.equal((ok as { action?: string }).action, "requeue");
    await assert.rejects(() => client.terminateRestartWork("w1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("claimWork empty poll unchanged vs miss (sak492-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { work: null, error: "queue empty" },
    { via: "broker_miss", work: null, error: "queue empty" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const empty = await client.claimWork("n1");
    assert.deepEqual(empty, { work: null, via: "broker" });
    await assert.rejects(() => client.claimWork("n1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("assertListOk empty vs miss (sak490-i / sak491-j)", () => {
  assert.deepEqual(SakClient.assertListOk({ work: [] }, "work"), { work: [] });
  assert.deepEqual(SakClient.assertListOk({ nodes: [] }, "nodes"), { nodes: [] });
  assert.throws(
    () => SakClient.assertListOk({ work: null }, "work"),
    /missing or non-list key work/,
  );
  assert.throws(
    () => SakClient.assertListOk({ nodes: null }, "nodes"),
    /missing or non-list key nodes/,
  );
  assert.throws(
    () =>
      SakClient.assertListOk(
        { via: "broker_miss", work: [], status: "degraded" },
        "work",
      ),
    /broker_miss/,
  );
  assert.throws(
    () =>
      SakClient.assertListOk(
        { via: "broker_miss", nodes: [], status: "degraded" },
        "nodes",
      ),
    /broker_miss/,
  );
});

test("listNodes empty vs miss (sak491-j)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { nodes: [] },
    { nodes: null },
    { via: "broker_miss", status: "degraded", feature: "list", nodes: [] },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const empty = await client.listNodes();
    assert.deepEqual((empty as { nodes?: unknown }).nodes, []);
    await assert.rejects(
      () => client.listNodes(),
      /missing or non-list key nodes/,
    );
    await assert.rejects(() => client.listNodes(), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("listNodesFiltered empty vs miss (sak491-j)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { nodes: [], action: "list" },
    { nodes: null, action: "list" },
    { via: "broker_miss", status: "degraded", feature: "list", nodes: [] },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const empty = await client.listNodesFiltered({ session_id: "s1" });
    assert.deepEqual((empty as { nodes?: unknown }).nodes, []);
    await assert.rejects(
      () => client.listNodesFiltered({ session_id: "s1" }),
      /missing or non-list key nodes/,
    );
    await assert.rejects(
      () => client.listNodesFiltered({ session_id: "s1" }),
      /broker_miss/,
    );
  } finally {
    globalThis.fetch = orig;
  }
});

test("registerNode record vs miss (sak491-j)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { node: { id: "n1", label: "w1" } },
    { node: null },
    { via: "broker_miss", status: "degraded", feature: "register" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.registerNode("w1");
    assert.equal((ok as { node?: { id?: string } }).node?.id, "n1");
    await assert.rejects(() => client.registerNode("w1"), /missing node record/);
    await assert.rejects(() => client.registerNode("w1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("heartbeatNode success vs miss (sak491-j)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { node: { id: "n1", label: "w1" }, action: "heartbeat" },
    { via: "broker_miss", status: "degraded", feature: "heartbeat" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.heartbeatNode("n1");
    assert.equal((ok as { action?: string }).action, "heartbeat");
    await assert.rejects(() => client.heartbeatNode("n1"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("assertCapacityOk empty vs miss (sak493-h)", () => {
  assert.deepEqual(SakClient.assertCapacityOk({}), {});
  assert.deepEqual(SakClient.assertCapacityOk({ ok: true }), { ok: true });
  assert.throws(
    () => SakClient.assertCapacityOk({ error: "down" }),
    /broker_miss/,
  );
  assert.throws(
    () =>
      SakClient.assertCapacityOk({
        via: "broker_miss",
        status: "degraded",
        feature: "health",
      }),
    /broker_miss/,
  );
  assert.throws(
    () =>
      SakClient.assertCapacityOk({
        via: "broker_miss",
        status: "degraded",
        feature: "capacity",
      }),
    /broker_miss/,
  );
});

test("assertListOk modules empty vs miss (sak493-h)", () => {
  assert.deepEqual(SakClient.assertListOk({ modules: [] }, "modules"), {
    modules: [],
  });
  assert.throws(
    () => SakClient.assertListOk({ modules: null }, "modules"),
    /missing or non-list key modules/,
  );
  assert.throws(
    () => SakClient.assertListOk({}, "modules"),
    /missing or non-list key modules/,
  );
  assert.throws(
    () =>
      SakClient.assertListOk(
        {
          via: "broker_miss",
          status: "degraded",
          feature: "list_modules",
        },
        "modules",
      ),
    /broker_miss/,
  );
});

test("assertRecordOk module empty vs miss (sak493-h)", () => {
  assert.deepEqual(
    SakClient.assertRecordOk({ module: { id: "m1" } }, "module"),
    { module: { id: "m1" } },
  );
  assert.deepEqual(SakClient.assertRecordOk({ id: "m1" }, "module"), {
    id: "m1",
  });
  assert.throws(
    () => SakClient.assertRecordOk({ module: null }, "module"),
    /missing module record/,
  );
  assert.throws(
    () => SakClient.assertRecordOk({}, "module"),
    /missing module record/,
  );
  assert.throws(
    () =>
      SakClient.assertRecordOk(
        {
          via: "broker_miss",
          status: "degraded",
          feature: "get_module",
        },
        "module",
      ),
    /broker_miss/,
  );
});

test("health empty vs miss (sak493-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    {},
    { via: "broker_miss", status: "degraded", feature: "health" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    assert.deepEqual(await client.health(), {});
    await assert.rejects(() => client.health(), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("listModules empty vs miss (sak493-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { modules: [] },
    { modules: null },
    {
      via: "broker_miss",
      status: "degraded",
      feature: "list_modules",
      modules: [],
    },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const empty = await client.listModules();
    assert.deepEqual((empty as { modules?: unknown }).modules, []);
    await assert.rejects(
      () => client.listModules(),
      /missing or non-list key modules/,
    );
    await assert.rejects(() => client.listModules(), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("getModule record vs miss (sak493-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    { module: { id: "demo" } },
    { module: null },
    { via: "broker_miss", status: "degraded", feature: "get_module" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    const ok = await client.getModule("demo");
    assert.equal((ok as { module?: { id?: string } }).module?.id, "demo");
    await assert.rejects(() => client.getModule("demo"), /missing module record/);
    await assert.rejects(() => client.getModule("demo"), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("capacity empty vs miss (sak493-h)", async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = mockFetch([
    {},
    { via: "broker_miss", status: "degraded", feature: "capacity" },
  ]);
  try {
    const client = new SakClient("http://127.0.0.1:8787");
    assert.deepEqual(await client.capacity(), {});
    await assert.rejects(() => client.capacity(), /broker_miss/);
  } finally {
    globalThis.fetch = orig;
  }
});

test("isMemoryMiss and assertMemoryOk empty vs miss (sak495-g)", () => {
  assert.equal(SakClient.isMemoryMiss({ code: "broker_memory_only" }), true);
  assert.equal(
    SakClient.isMemoryMiss({
      via: "broker_miss",
      status: "degraded",
      feature: "fleet_memory_search",
      hits: [],
    }),
    true,
  );
  assert.equal(
    SakClient.isMemoryMiss({ feature: "fleet_memory_search", error: "down", hits: [] }),
    true,
  );
  assert.equal(SakClient.isMemoryMiss({ hits: [], via: "broker" }), false);

  assert.deepEqual(SakClient.assertMemoryOk({ hits: [] }), { hits: [] });
  assert.deepEqual(
    SakClient.assertMemoryOk({ hits: [{ id: "m1" }], via: "broker" }),
    { hits: [{ id: "m1" }], via: "broker" },
  );
  assert.throws(() => SakClient.assertMemoryOk({ hits: null }), /missing or non-list key hits/);
  assert.throws(() => SakClient.assertMemoryOk({}), /missing or non-list key hits/);
  assert.throws(
    () =>
      SakClient.assertMemoryOk({
        via: "broker_miss",
        status: "degraded",
        feature: "fleet_memory_search",
        hits: [],
      }),
    /broker_miss/,
  );
  assert.throws(
    () =>
      SakClient.assertMemoryOk({
        error: "down",
        feature: "fleet_memory_search",
        hits: [],
      }),
    /broker_miss/,
  );
});

test("domain miss detectors and asserts (sak496-i)", () => {
  assert.equal(SakClient.isSandboxMiss({ code: "broker_sandbox_only" }), true);
  assert.equal(
    SakClient.isSandboxMiss({ via: "broker_miss", feature: "sandbox_exec", error: "down" }),
    true,
  );
  assert.equal(SakClient.isSandboxMiss({ stdout: "ok", via: "broker" }), false);

  assert.equal(SakClient.isToolsMiss({ code: "broker_tools_only" }), true);
  assert.equal(
    SakClient.isToolsMiss({ via: "broker_miss", feature: "shell", error: "down" }),
    true,
  );

  assert.equal(SakClient.isResearchMiss({ code: "broker_research_only" }), true);
  assert.equal(
    SakClient.isResearchMiss({ via: "broker_miss", feature: "research_fetch", error: "down" }),
    true,
  );

  assert.equal(SakClient.isEgressMiss({ code: "broker_egress_only" }), true);
  assert.equal(
    SakClient.isEgressMiss({ via: "broker_miss", feature: "egress_audit", error: "down" }),
    true,
  );

  assert.equal(SakClient.isLlmMiss({ code: "broker_llm_unavailable" }), true);
  assert.equal(
    SakClient.isLlmMiss({ via: "broker_miss", feature: "llm", error: "down" }),
    true,
  );
  assert.equal(SakClient.isLlmMiss({ content: "hi", via: "broker" }), false);

  assert.deepEqual(SakClient.assertSandboxOk({ stdout: "ok" }), { stdout: "ok" });
  assert.deepEqual(SakClient.assertLlmOk({ content: "hi" }), { content: "hi" });
  assert.throws(
    () => SakClient.assertSandboxOk({ via: "broker_miss", feature: "sandbox_exec" }),
    /broker_miss/,
  );
  assert.throws(() => SakClient.assertLlmOk({ code: "broker_llm_unavailable" }), /broker_miss/);
});
