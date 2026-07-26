/** SwissArmyNoife HTTP admin + MCP clients (`sak321` / `sak428-f` / `sak490-i` / `sak491-j` / `sak492-h`).

 * Rust `sdk` is HTTP-only. `SakMcpClient` adds MCP `compute_work` / `claimWork`;
 * work-write empty-vs-miss asserts live on this HTTP `SakClient` (`sak492-h`).
 */

export { SakMcpClient, type FetchFn, type SakMcpClientOptions } from "./mcp.js";

/** Loose JSON from admin endpoints (shape stabilizes with OpenAPI). */
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export type HealthResponse = JsonValue;
export type ModuleListResponse = JsonValue;
export type ModuleResponse = JsonValue;
export type CapacityResponse = JsonValue;
export type WorkListResponse = JsonValue;
export type NodeListResponse = JsonValue;

export class SakClient {
  readonly baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }

  async health(): Promise<HealthResponse> {
    return SakClient.assertCapacityOk(  // sak485-i / sak486-i / sak493-h
      await this.getJson("/health"),
    );
  }

  async listModules(): Promise<ModuleListResponse> {
    return SakClient.assertListOk(  // sak484-i / sak485-i / sak493-h
      await this.getJson("/v1/sak/modules"),
      "modules",
    );
  }

  async getModule(id: string): Promise<ModuleResponse> {
    return SakClient.assertRecordOk(  // sak485-i / sak493-h
      await this.getJson(`/v1/sak/modules/${encodeURIComponent(id)}`),
      "module",
    );
  }

  async capacity(): Promise<CapacityResponse> {
    return SakClient.assertCapacityOk(  // sak486-i / sak493-h
      await this.getJson("/v1/sak/capacity"),
    );
  }

  /** `POST /v1/chat/completions` OpenAI-shaped facade (`sak546-a`). */
  async chatCompletions(body: Record<string, JsonValue>): Promise<JsonValue> {
    return this.postJson("/v1/chat/completions", body);
  }

  async listWork(): Promise<WorkListResponse> {
    return SakClient.assertListOk(
      await this.getJson("/v1/sak/compute/work"),
      "work",
    );
  }

  async listNodes(): Promise<NodeListResponse> {
    return SakClient.assertListOk(  // sak491-j
      await this.getJson("/v1/sak/compute/nodes"),
      "nodes",
    );
  }

  async computeWork(body: Record<string, JsonValue>): Promise<JsonValue> {
    return SakClient.assertRawComputePost(
      await this.postJson("/v1/sak/compute/work", body),
      body,
    );
  }

  async computeNodes(body: Record<string, JsonValue>): Promise<JsonValue> {
    return SakClient.assertRawComputePost(
      await this.postJson("/v1/sak/compute/nodes", body),
      body,
    );
  }

  async registerNode(
    label: string,
    opts?: { caps?: string[]; nodeId?: string; sessionId?: string },
  ): Promise<JsonValue> {
    const body: Record<string, JsonValue> = { action: "register", label };
    if (opts?.caps) body.caps = opts.caps;
    if (opts?.nodeId) body.node_id = opts.nodeId;
    if (opts?.sessionId) body.session_id = opts.sessionId;
    return SakClient.assertRecordOk(await this.computeNodes(body), "node");  // sak484-i / sak487-i / sak491-j
  }

  async heartbeatNode(nodeId: string): Promise<JsonValue> {
    return SakClient.assertRecordOk(  // sak484-i / sak487-i / sak491-j
      await this.computeNodes({ action: "heartbeat", node_id: nodeId }),
      "node",
    );
  }

  async requeueWork(workId: string): Promise<JsonValue> {
    return SakClient.assertRecordOk(  // sak483-i / sak487-i / sak492-h
      await this.computeWork({ action: "requeue", work_id: workId }),
      "work",
    );
  }

  /** Alias for terminate-restart / requeue (`sak480-i` / `sak492-h`). */
  async terminateRestartWork(workId: string): Promise<JsonValue> {
    return this.requeueWork(workId);  // sak484-i / sak485-i / sak492-h
  }

  /** Queued work count via filtered list (`sak481-i`; payload session_id first). */
  async queueDepth(sessionId?: string): Promise<JsonValue> {
    const raw = await this.listWorkFiltered({ status: "queued", limit: 200 });
    const obj = raw as Record<string, JsonValue>;
    const work = Array.isArray(obj.work) ? obj.work : [];
    const items = work.filter(
      (w) => w && typeof w === "object" && !Array.isArray(w),
    ) as Record<string, JsonValue>[];
    return {
      queued: SakClient.queueDepthForSession(items, sessionId),
      session_id: sessionId ?? null,
      via: "broker",
      status: "ok",
    };
  }

  /** Session nodes + queue depth via broker lists (`sak482-i` / `sak485-i` / `sak494-i`). */
  async sessionComputeStatus(
    sessionId?: string,
    opts?: { feature?: string },
  ): Promise<JsonValue> {
    const nodeFilters: Record<string, JsonValue> = {};
    if (sessionId) nodeFilters.session_id = sessionId;
    const nodesRaw = await this.listNodesFiltered(nodeFilters);
    const nodesObj = nodesRaw as Record<string, JsonValue>;
    const rawNodes = Array.isArray(nodesObj.nodes) ? nodesObj.nodes : [];
    const nodes = rawNodes
      .filter((n) => n && typeof n === "object" && !Array.isArray(n))
      .map((n) => {
        const item = n as Record<string, JsonValue>;
        const caps = Array.isArray(item.caps) ? item.caps : [];
        const capabilities: Record<string, JsonValue> = {};
        for (const c of caps) {
          if (typeof c === "string") capabilities[c] = true;
        }
        return {
          node_id: SakClient.nodeIdFromBrokerRecord(item),
          display_name: item.label ?? null,
          host_label: item.label ?? null,
          status: "online",
          capabilities,
          session_id: item.session_id ?? null,
          via: "broker",
        };
      });

    try {
      const workRaw = await this.listWorkFiltered({ status: "queued", limit: 200 });
      const workObj = workRaw as Record<string, JsonValue>;
      const work = Array.isArray(workObj.work) ? workObj.work : [];
      const items = work.filter(
        (w) => w && typeof w === "object" && !Array.isArray(w),
      ) as Record<string, JsonValue>[];

      const out: Record<string, JsonValue> = {
        nodes,
        queue_depth: SakClient.queueDepthForSession(items, sessionId),
        via: "broker",
        status: "ok",
      };
      if (sessionId != null) out.session_id = sessionId;
      if (opts?.feature) out.feature = opts.feature;
      return out;
    } catch (exc) {
      return SakClient.brokerSessionQueueMiss(exc, nodes, sessionId, opts?.feature);
    }
  }

  /** Nodes-ok + queue-fail → broker_miss/degraded (never via=broker success) (`sak494-i`). */
  static brokerSessionQueueMiss(
    exc: unknown,
    nodes: JsonValue[],
    sessionId?: string,
    feature?: string,
  ): Record<string, JsonValue> {
    let err = exc instanceof Error ? exc.message : String(exc);
    if (err.startsWith("broker_miss:")) {
      const parts = err.split(":", 3);
      if (parts.length >= 3) err = parts[2].trim();
    }
    const out: Record<string, JsonValue> = {
      nodes,
      queue_depth: 0,
      via: "broker_miss",
      status: "degraded",
      error: err,
    };
    if (sessionId != null) out.session_id = sessionId;
    if (feature) out.feature = feature;
    return out;
  }

  /** Session id from broker work unit payload (`sak481-i`). */
  static workSessionId(work: Record<string, JsonValue>): string {
    const payload = work.payload;
    if (payload && typeof payload === "object" && !Array.isArray(payload)) {
      const sid = (payload as Record<string, JsonValue>).session_id;
      if (sid != null) return String(sid);
    }
    if (work.session_id != null) return String(work.session_id);
    return "";
  }

  /** Extract node id from HTTP/MCP node records (`sak482-i`). */
  static nodeIdFromBrokerRecord(node: Record<string, JsonValue>): string {
    const nodeId = node.node_id ?? node.id;
    return nodeId == null ? "" : String(nodeId);
  }

  /** Count queued work; optionally filter by session_id (`sak481-i`). */
  static queueDepthForSession(
    workItems: Record<string, JsonValue>[],
    sessionId?: string,
  ): number {
    if (!sessionId) return workItems.length;
    return workItems.filter((w) => SakClient.workSessionId(w) === sessionId).length;
  }

  async enqueueWork(
    kind: string,
    payload?: Record<string, JsonValue>,
  ): Promise<JsonValue> {
    return SakClient.assertRecordOk(  // sak483-i / sak487-i / sak492-h
      await this.computeWork({
        action: "enqueue",
        kind,
        payload: payload ?? {},
      }),
      "work",
    );
  }

  async claimWork(nodeId: string): Promise<JsonValue> {
    return SakClient.normalizeClaimWorkResponse(
      await this.computeWork({ action: "claim", node_id: nodeId }),
    );
  }

  /** True when body is a structured compute peel miss (`sak483-i`). */
  static isComputeMiss(raw: JsonValue): boolean {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      return false;
    }
    const obj = raw as Record<string, JsonValue>;
    if (obj.via === "broker_miss" || obj.status === "degraded") {
      return true;
    }
    if (obj.error != null && String(obj.error).length > 0) {
      return true;
    }
    return false;
  }

  /** Shared domain peel miss detector (`sak496-i`). */
  private static featureDomainMiss(
    raw: JsonValue,
    domainCode: string,
    keywords: string[],
  ): boolean {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      return false;
    }
    const obj = raw as Record<string, JsonValue>;
    if (obj.code === domainCode) {
      return true;
    }
    if (SakClient.isComputeMiss(obj)) {
      return true;
    }
    const feat = obj.feature;
    if (typeof feat === "string") {
      const low = feat.toLowerCase();
      if (keywords.some((kw) => low.includes(kw))) {
        if (obj.via === "broker_miss") {
          return true;
        }
        if (obj.error != null && String(obj.error).length > 0) {
          return true;
        }
      }
    }
    return false;
  }

  /** Domain assert: peel miss or error dict raises (`sak496-i`). */
  private static assertDomainOk(
    raw: JsonValue,
    isMiss: (body: JsonValue) => boolean,
  ): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      throw new Error("broker_miss: non-object response");
    }
    const obj = raw as Record<string, JsonValue>;
    if (isMiss(obj)) {
      throw new Error(
        `broker_miss: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    if (obj.error != null) {
      throw new Error(`broker_miss: ${String(obj.error)}`);
    }
    return obj;
  }

  /** True when body is a structured memory peel miss (`sak480-g` / `sak493-i` / `sak495-g`). */
  static isMemoryMiss(raw: JsonValue): boolean {
    return SakClient.featureDomainMiss(raw, "broker_memory_only", ["memory"]);
  }

  /** True when body is a structured sandbox peel miss (`sak496-i`). */
  static isSandboxMiss(raw: JsonValue): boolean {
    return SakClient.featureDomainMiss(raw, "broker_sandbox_only", ["sandbox"]);
  }

  /** True when body is a structured tools peel miss (`sak496-i`). */
  static isToolsMiss(raw: JsonValue): boolean {
    return SakClient.featureDomainMiss(raw, "broker_tools_only", ["tools", "shell"]);
  }

  /** True when body is a structured research peel miss (`sak496-i`). */
  static isResearchMiss(raw: JsonValue): boolean {
    return SakClient.featureDomainMiss(raw, "broker_research_only", ["research"]);
  }

  /** True when body is a structured egress peel miss (`sak496-i`). */
  static isEgressMiss(raw: JsonValue): boolean {
    return SakClient.featureDomainMiss(raw, "broker_egress_only", ["egress"]);
  }

  /** True when body is a structured LLM peel miss (`sak496-i`). */
  static isLlmMiss(raw: JsonValue): boolean {
    return SakClient.featureDomainMiss(raw, "broker_llm_unavailable", ["llm"]);
  }

  /** Sandbox assert: peel miss or error dict raises (`sak496-i`). */
  static assertSandboxOk(raw: JsonValue): JsonValue {
    return SakClient.assertDomainOk(raw, SakClient.isSandboxMiss);
  }

  /** Tools assert: peel miss or error dict raises (`sak496-i`). */
  static assertToolsOk(raw: JsonValue): JsonValue {
    return SakClient.assertDomainOk(raw, SakClient.isToolsMiss);
  }

  /** Research assert: peel miss or error dict raises (`sak496-i`). */
  static assertResearchOk(raw: JsonValue): JsonValue {
    return SakClient.assertDomainOk(raw, SakClient.isResearchMiss);
  }

  /** Egress assert: peel miss or error dict raises (`sak496-i`). */
  static assertEgressOk(raw: JsonValue): JsonValue {
    return SakClient.assertDomainOk(raw, SakClient.isEgressMiss);
  }

  /** LLM assert: peel miss or error dict raises (`sak496-i`). */
  static assertLlmOk(raw: JsonValue): JsonValue {
    return SakClient.assertDomainOk(raw, SakClient.isLlmMiss);
  }

  /** Memory search assert: error or non-list hits raises; empty `[]` ok (`sak495-g`). */
  static assertMemoryOk(
    raw: JsonValue,
    listKey: "hits" | "results" = "hits",
  ): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      throw new Error(`broker_miss: non-object response for ${listKey}`);
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
    if (!Array.isArray(obj[listKey])) {
      throw new Error(`broker_miss: missing or non-list key ${listKey}`);
    }
    return obj;
  }

  /**
   * Normalize claim response: empty queue → `{ work: null, via: "broker" }`;
   * ``via=broker_miss`` always throws (`sak440-h` / `sak488-i` / `sak490-i` / `sak492-h` claim unchanged).
   */
  static normalizeClaimWorkResponse(raw: JsonValue): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      throw new Error("broker_miss: claim: non-object response");
    }
    const obj = raw as Record<string, JsonValue>;
    if (obj.via === "broker_miss" || obj.status === "degraded") {  // sak488-i
      throw new Error(
        `broker_miss: claim: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    const work = obj.work;
    const err = obj.error;
    const errStr = err == null ? "" : String(err);
    const emptyPoll =
      work == null &&
      (err == null ||
        errStr.toLowerCase().includes("empty") ||
        errStr.toLowerCase().includes("no work"));
    if (emptyPoll) {
      return { work: null, via: "broker" };
    }
    if (SakClient.isComputeMiss(obj)) {
      throw new Error(
        `broker_miss: claim: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    if (err != null) {
      throw new Error(`broker_miss: claim: ${errStr}`);
    }
    if (work && typeof work === "object" && !Array.isArray(work)) {
      return obj;
    }
    if (obj.id != null) {
      return obj;
    }
    throw new Error("broker_miss: claim: missing work record");
  }

  /** List assert: error or non-list key raises (`sak441-f` / `sak488-i` / `sak490-i` / `sak491-j`). */
  static assertListOk(
    raw: JsonValue,
    listKey: "nodes" | "work" | "modules" = "nodes",
  ): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      throw new Error(`broker_miss: non-object response for ${listKey}`);
    }
    const obj = raw as Record<string, JsonValue>;
    if (SakClient.isComputeMiss(obj)) {  // sak488-i
      throw new Error(
        `broker_miss: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    if (obj.error != null) {
      throw new Error(`broker_miss: ${String(obj.error)}`);
    }
    if (!Array.isArray(obj[listKey])) {
      throw new Error(`broker_miss: missing or non-list key ${listKey}`);
    }
    return obj;
  }

  /** Single-record assert for write/get helpers (`sak445-g` / `sak446-f` / `sak487-i` / `sak491-j` / `sak492-h`). */
  static assertRecordOk(
    raw: JsonValue,
    recordKey: "work" | "node" | "module" = "work",
  ): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      throw new Error(`broker_miss: non-object for ${recordKey}`);
    }
    const obj = raw as Record<string, JsonValue>;
    if (SakClient.isComputeMiss(obj)) {  // sak487-i / sak492-h
      throw new Error(
        `broker_miss: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    if (obj.error != null) {
      throw new Error(`broker_miss: ${String(obj.error)}`);
    }
    const rec = obj[recordKey];
    if (rec && typeof rec === "object" && !Array.isArray(rec)) {
      return obj;
    }
    if (recordKey === "work" && obj.id != null && obj.action == null) {
      return obj;
    }
    if (
      recordKey === "node" &&
      (obj.id != null || obj.node_id != null) &&
      obj.nodes == null
    ) {
      return obj;
    }
    if (recordKey === "module" && obj.id != null && obj.modules == null) {
      return obj;
    }
    throw new Error(`broker_miss: missing ${recordKey} record`);
  }

  /** Capacity assert: error dict raises (`sak446-f` / `sak485-i` / `sak486-i` / `sak493-h`).

   * Empty object `{}` is success; peel miss / error field is hard miss.
   */
  static assertCapacityOk(raw: JsonValue): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      throw new Error("broker_miss: non-object for capacity");
    }
    const obj = raw as Record<string, JsonValue>;
    if (SakClient.isComputeMiss(obj)) {  // sak485-i / sak486-i
      throw new Error(
        `broker_miss: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    if (obj.error != null) {
      throw new Error(`broker_miss: ${String(obj.error)}`);
    }
    return obj;
  }

  /** Raw compute POST: error raises except claim (`sak447-g`). */
  static assertRawComputePost(
    raw: JsonValue,
    body: Record<string, JsonValue>,
  ): JsonValue {
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      return raw;
    }
    if (String(body.action || "") === "claim") {
      return raw;
    }
    const obj = raw as Record<string, JsonValue>;
    if (SakClient.isComputeMiss(obj)) {
      throw new Error(
        `broker_miss: ${String(obj.error ?? obj.feature ?? obj.via ?? "miss")}`,
      );
    }
    if (obj.error != null) {
      throw new Error(`broker_miss: ${String(obj.error)}`);
    }
    return obj;
  }

  async completeWork(
    workId: string,
    nodeId: string,
    result?: Record<string, JsonValue>,
  ): Promise<JsonValue> {
    return SakClient.assertRecordOk(  // sak483-i / sak487-i / sak492-h
      await this.computeWork({
        action: "complete",
        work_id: workId,
        node_id: nodeId,
        result: result ?? {},
      }),
      "work",
    );
  }

  async getWork(workId: string): Promise<JsonValue> {
    return SakClient.assertRecordOk(  // sak483-i / sak487-i / sak492-h
      await this.computeWork({ action: "get", work_id: workId }),
      "work",
    );
  }

  async listWorkFiltered(
    filters: Record<string, JsonValue> = {},
  ): Promise<JsonValue> {
    return SakClient.assertListOk(  // sak488-i / sak490-i
      await this.computeWork({ action: "list", ...filters }),
      "work",
    );
  }

  async listNodesFiltered(
    filters: Record<string, JsonValue> = {},
  ): Promise<JsonValue> {
    return SakClient.assertListOk(  // sak491-j
      await this.computeNodes({ action: "list", ...filters }),
      "nodes",
    );
  }

  private async getJson(path: string): Promise<JsonValue> {
    const res = await fetch(`${this.baseUrl}${path}`);
    if (!res.ok) {
      const body = await res.text();
      throw new Error(`${res.status}: ${body}`);
    }
    return res.json() as Promise<JsonValue>;
  }

  private async postJson(
    path: string,
    payload: Record<string, JsonValue>,
  ): Promise<JsonValue> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!res.ok) {
      const body = await res.text();
      throw new Error(`${res.status}: ${body}`);
    }
    return res.json() as Promise<JsonValue>;
  }
}
