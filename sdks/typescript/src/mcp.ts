/** Streamable HTTP MCP client (`sak321-d` / `sak329-a` / `sak489-i` / `sak490-i`).

 * MCP exposes `computeWork` + `claimWork` only. List/get/requeue empty-vs-miss
 * asserts are on HTTP `SakClient`; Rust `sdk` is HTTP-only.
 */

import { SakClient, type JsonValue } from "./index.js";

export type FetchFn = typeof fetch;

export type SakMcpClientOptions = {
  token?: string;
  fetch?: FetchFn;
  /** When false, skip auto ``initialize`` (tests / raw RPC). Default true. */
  autoInitialize?: boolean;
};

type JsonRpcResponse = {
  result?: unknown;
  error?: { message?: string };
};

const DEFAULT_MCP_URL = "http://127.0.0.1:8080/mcp";
const MCP_PROTOCOL_VERSION = "2024-11-05";
const SESSION_HEADER = "mcp-session-id";
const MCP_ACCEPT = "application/json, text/event-stream";

export type MemoryDocument = { id: string; text: string };

export class SakMcpClient {
  /** Memory peel miss detector (`sak495-g` / `sak499-i`). */
  static isMemoryMiss(raw: JsonValue): boolean {
    return SakClient.isMemoryMiss(raw);
  }

  /** Memory search assert: peel miss or non-list hits raises (`sak495-g` / `sak499-i`). */
  static assertMemoryOk(
    raw: JsonValue,
    listKey: "hits" | "results" = "hits",
  ): JsonValue {
    return SakClient.assertMemoryOk(raw, listKey);
  }

  readonly baseUrl: string;
  private readonly token?: string;
  private readonly fetchFn: FetchFn;
  private readonly autoInitialize: boolean;
  private rpcId = 0;
  private sessionId: string | null = null;
  private initialized = false;

  constructor(baseUrl = DEFAULT_MCP_URL, options?: SakMcpClientOptions) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.token = options?.token;
    this.fetchFn = options?.fetch ?? fetch;
    this.autoInitialize = options?.autoInitialize !== false;
  }

  /** Current Streamable HTTP session id after ``initialize`` (`sak329-a`). */
  getSessionId(): string | null {
    return this.sessionId;
  }

  /**
   * MCP ``initialize`` + ``notifications/initialized``; capture ``mcp-session-id``
   * from response headers or body (`sak329-a`).
   */
  async initialize(): Promise<unknown> {
    const { result, sessionId } = await this.rpcRaw("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      capabilities: {},
      clientInfo: { name: "@swissarmynoife/sdk", version: "0.1.0" },
    });
    if (sessionId) {
      this.sessionId = sessionId;
    }
    await this.postNotification("notifications/initialized");
    this.initialized = true;
    return result;
  }

  async ensureSession(): Promise<void> {
    if (!this.autoInitialize || this.initialized) {
      return;
    }
    await this.initialize();
  }

  async ping(): Promise<string> {
    const result = await this.toolsCall("ping");
    return extractPingText(result);
  }

  async toolsList(): Promise<unknown> {
    await this.ensureSession();
    return this.rpc("tools/list");
  }

  async catalogList(): Promise<unknown> {
    return this.toolsCall("catalog_list");
  }

  /** MCP ``compute_work`` — raw invoke body; claim skips hard assert (`sak489-i`). */
  async computeWork(body: Record<string, JsonValue>): Promise<JsonValue> {
    const raw = unwrapMcpComputePayload(
      await this.toolsCall("compute_work", body as Record<string, unknown>),
    );
    return SakClient.assertRawComputePost(raw, body); // sak489-i
  }

  /** MCP ``compute_work`` claim + shared empty-vs-miss normalize (`sak489-i`). */
  async claimWork(nodeId: string, bindingId: string): Promise<JsonValue> {
    return SakClient.normalizeClaimWorkResponse(
      // sak488-i / sak489-i / sak490-i
      await this.computeWork({
        binding_id: bindingId,
        action: "claim",
        node_id: nodeId,
      }),
    );
  }

  /** MCP ``sandbox_exec`` with domain peel assert (`sak498-g`). */
  async sandboxExec(argv: string[], cwd = "."): Promise<JsonValue> {
    return SakClient.assertSandboxOk(
      await this.domainToolRaw("sandbox_exec", { argv, cwd }),
    );
  }

  /** MCP ``shell_exec`` with domain peel assert (`sak498-g`). */
  async shellExec(argv: string[], cwd = "."): Promise<JsonValue> {
    return SakClient.assertToolsOk(
      await this.domainToolRaw("shell_exec", { argv, cwd }),
    );
  }

  /** MCP ``research_fetch`` with domain peel assert (`sak498-g`). */
  async researchFetch(url: string): Promise<JsonValue> {
    return SakClient.assertResearchOk(
      await this.domainToolRaw("research_fetch", { url }),
    );
  }

  /** MCP ``egress_check`` with domain peel assert (`sak498-g`). */
  async egressCheck(url: string): Promise<JsonValue> {
    return SakClient.assertEgressOk(
      await this.domainToolRaw("egress_check", { url }),
    );
  }

  /** MCP ``llm_chat`` with domain peel assert (`sak498-g`). */
  async llmChat(
    messages: Record<string, JsonValue>[],
    model?: string,
  ): Promise<JsonValue> {
    const args: Record<string, JsonValue> = { messages };
    if (model !== undefined) {
      args.model = model;
    }
    return SakClient.assertLlmOk(await this.domainToolRaw("llm_chat", args));
  }

  /** MCP ``memory_index`` with domain peel assert (`sak499-i`). */
  async memoryIndex(
    bindingId: string,
    documents: MemoryDocument[],
    scopeKey?: string,
  ): Promise<JsonValue> {
    const args: Record<string, JsonValue> = {
      binding_id: bindingId,
      documents,
    };
    if (scopeKey !== undefined) {
      args.scope_key = scopeKey;
    }
    return assertMemoryPeelOk(await this.domainToolRaw("memory_index", args));
  }

  private async domainToolRaw(
    name: string,
    args: Record<string, JsonValue>,
  ): Promise<JsonValue> {
    const raw = await this.toolsCall(name, args as Record<string, unknown>);
    return raw && typeof raw === "object" && !Array.isArray(raw)
      ? (raw as JsonValue)
      : { result: raw as JsonValue };
  }

  private baseHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      Accept: MCP_ACCEPT,
    };
    if (this.token) {
      headers.Authorization = `Bearer ${this.token}`;
    }
    if (this.sessionId) {
      headers[SESSION_HEADER] = this.sessionId;
    }
    return headers;
  }

  private async postNotification(method: string): Promise<void> {
    const res = await this.fetchFn(this.baseUrl, {
      method: "POST",
      headers: this.baseHeaders(),
      body: JSON.stringify({
        jsonrpc: "2.0",
        method,
      }),
    });
    if (res.status !== 200 && res.status !== 202) {
      const body = await res.text();
      throw new Error(`${res.status}: ${body}`);
    }
  }

  private async rpcRaw(
    method: string,
    params: Record<string, unknown> = {},
  ): Promise<{ result: unknown; sessionId: string | null }> {
    this.rpcId += 1;
    const res = await this.fetchFn(this.baseUrl, {
      method: "POST",
      headers: this.baseHeaders(),
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: this.rpcId,
        method,
        params,
      }),
    });

    if (!res.ok) {
      const body = await res.text();
      throw new Error(`${res.status}: ${body}`);
    }

    const headerSid = res.headers.get(SESSION_HEADER);
    const body = (await res.json()) as JsonRpcResponse & Record<string, unknown>;
    if (body.error) {
      throw new Error(
        (body.error as { message?: string }).message ?? `MCP ${method} failed`,
      );
    }
    const fromBody = sessionIdFromBody(body);
    return {
      result: body.result,
      sessionId: (headerSid && headerSid.trim()) || fromBody,
    };
  }

  private async rpc(
    method: string,
    params: Record<string, unknown> = {},
  ): Promise<unknown> {
    const { result, sessionId } = await this.rpcRaw(method, params);
    if (sessionId && !this.sessionId) {
      this.sessionId = sessionId;
    }
    return result;
  }

  private async toolsCall(
    name: string,
    args: Record<string, unknown> = {},
  ): Promise<unknown> {
    await this.ensureSession();
    return this.rpc("tools/call", { name, arguments: args });
  }
}

function sessionIdFromBody(body: Record<string, unknown>): string | null {
  for (const key of ["sessionId", "session_id", "mcp-session-id"] as const) {
    const val = body[key];
    if (typeof val === "string" && val.trim()) {
      return val.trim();
    }
  }
  const result = body.result;
  if (result && typeof result === "object" && !Array.isArray(result)) {
    const r = result as Record<string, unknown>;
    for (const key of ["sessionId", "session_id", "mcp-session-id"] as const) {
      const val = r[key];
      if (typeof val === "string" && val.trim()) {
        return val.trim();
      }
    }
  }
  return null;
}

/** Unwrap MCP tool content + InvokeResp into HTTP-shaped compute JSON (`sak489-i`). */
function unwrapMcpComputePayload(result: unknown): JsonValue {
  let payload: unknown = result;
  if (payload && typeof payload === "object" && !Array.isArray(payload)) {
    const obj = payload as Record<string, unknown>;
    const content = obj.content;
    if (Array.isArray(content)) {
      for (const item of content) {
        if (item && typeof item === "object") {
          const text = (item as { text?: unknown }).text;
          if (typeof text === "string") {
            try {
              payload = JSON.parse(text) as unknown;
            } catch {
              payload = text;
            }
            break;
          }
        }
      }
    }
  }
  if (payload && typeof payload === "object" && !Array.isArray(payload)) {
    const inv = payload as Record<string, JsonValue>;
    if (inv.status === "ok" && "result" in inv) {
      return inv.result ?? null;
    }
    if (inv.status === "error") {
      return {
        work: null,
        error: inv.message ?? "invoke error",
      };
    }
  }
  return payload as JsonValue;
}

/** Memory domain peel assert for index/search transports (`sak499-i`). */
function assertMemoryPeelOk(raw: JsonValue): JsonValue {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("broker_miss: non-object response");
  }
  const obj = raw as Record<string, JsonValue>;
  if (SakClient.isMemoryMiss(obj)) {
    throw new Error(
      `broker_miss: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
    );
  }
  if (obj.error != null) {
    throw new Error(`broker_miss: ${String(obj.error)}`);
  }
  return obj;
}

function extractPingText(result: unknown): string {
  if (typeof result === "string") {
    return result;
  }
  if (result && typeof result === "object") {
    const content = (result as { content?: unknown }).content;
    if (Array.isArray(content)) {
      for (const item of content) {
        if (item && typeof item === "object") {
          const text = (item as { text?: unknown }).text;
          if (typeof text === "string") {
            return text;
          }
        }
      }
    }
  }
  return JSON.stringify(result ?? "");
}
