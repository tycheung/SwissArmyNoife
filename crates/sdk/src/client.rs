//! `SakClient` — thin HTTP wrapper over `http-admin` (`sak320-a/b/c`).
//! HTTP-only: no MCP transport. TS/Python `SakMcpClient` adds MCP `compute_work` /
//! `claim_work`; work-write empty-vs-miss asserts stay on this HTTP surface (`sak490-i` / `sak492-h`);
//! node-path list/register/heartbeat parity (`sak491-j`).

use serde_json::Value;

use crate::SdkError;

/// Ergonomic client for SwissArmyNoife HTTP admin endpoints.
#[derive(Clone, Debug)]
pub struct SakClient {
    base: String,
    http: reqwest::Client,
}

impl SakClient {
    /// Create a client for `base_url` (e.g. `http://127.0.0.1:8787`).
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base: base_url.into().trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Base URL without trailing slash.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// `GET /health`
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / peel error dict (`sak449-i`).
    pub async fn health(&self) -> Result<Value, SdkError> {
        let raw = self.get_json("/health").await?;
        assert_capacity_ok(&raw) // sak485-i / sak486-i / sak493-h
    }

    /// `GET /v1/sak/modules`
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / list assert (`sak446-e`).
    pub async fn list_modules(&self) -> Result<Value, SdkError> {
        let raw = self.get_json("/v1/sak/modules").await?;
        assert_list_ok(&raw, "modules") // sak484-i / sak485-i / sak493-h
    }

    /// `GET /v1/sak/modules/{id}`
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert (`sak446-e`).
    pub async fn get_module(&self, id: &str) -> Result<Value, SdkError> {
        let raw = self.get_json(&format!("/v1/sak/modules/{id}")).await?;
        assert_record_ok(&raw, "module") // sak485-i / sak493-h
    }

    /// `GET /v1/sak/capacity`
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / capacity assert (`sak446-e`).
    pub async fn capacity(&self) -> Result<Value, SdkError> {
        let raw = self.get_json("/v1/sak/capacity").await?;
        assert_capacity_ok(&raw) // sak486-i / sak493-h
    }

    /// `GET /v1/sak/compute/work`
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / list assert (`sak443-h`).
    pub async fn list_work(&self) -> Result<Value, SdkError> {
        let raw = self.get_json("/v1/sak/compute/work").await?;
        assert_list_ok(&raw, "work")
    }

    /// `POST /v1/sak/compute/work` enqueue/claim/complete/get/list.
    ///
    /// Prefer typed helpers. Hard error dicts raise except claim empty-polls (`sak447-g`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / peel error dict.
    pub async fn compute_work(&self, body: Value) -> Result<Value, SdkError> {
        let raw = self.post_json("/v1/sak/compute/work", body.clone()).await?;
        assert_raw_compute_post(&raw, &body)
    }

    /// `GET /v1/sak/compute/nodes`
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / list assert (`sak443-h`).
    pub async fn list_nodes(&self) -> Result<Value, SdkError> {
        let raw = self.get_json("/v1/sak/compute/nodes").await?;
        assert_list_ok(&raw, "nodes") // sak491-j
    }

    /// `POST /v1/sak/compute/nodes` register/heartbeat/list (`sak424-h`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse.
    /// `POST /v1/sak/compute/nodes` register/heartbeat/list.
    ///
    /// Prefer typed helpers. Hard error dicts raise (`sak447-g`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / peel error dict.
    pub async fn compute_nodes(&self, body: Value) -> Result<Value, SdkError> {
        let raw = self
            .post_json("/v1/sak/compute/nodes", body.clone())
            .await?;
        assert_raw_compute_post(&raw, &body)
    }

    /// `POST /v1/sak/compute/nodes` filtered list (`sak425-f` / `sak433-e` / `sak443-h`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / list assert.
    pub async fn list_nodes_filtered(&self, body: Value) -> Result<Value, SdkError> {
        let mut payload = body;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("action".into(), Value::String("list".into()));
        }
        let raw = self.post_json("/v1/sak/compute/nodes", payload).await?;
        assert_list_ok(&raw, "nodes") // sak491-j
    }

    /// `POST /v1/sak/compute/nodes` register (`sak427-f` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn register_node(&self, body: Value) -> Result<Value, SdkError> {
        let raw = self.post_json("/v1/sak/compute/nodes", body).await?;
        assert_record_ok(&raw, "node") // sak484-i / sak487-i / sak491-j
    }

    /// `POST /v1/sak/compute/nodes` heartbeat (`sak427-f` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn heartbeat_node(&self, body: Value) -> Result<Value, SdkError> {
        let raw = self.post_json("/v1/sak/compute/nodes", body).await?;
        assert_record_ok(&raw, "node") // sak484-i / sak487-i / sak491-j
    }

    /// `POST /v1/sak/compute/work` requeue (`sak429-b` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn requeue_work(&self, work_id: &str) -> Result<Value, SdkError> {
        let raw = self
            .compute_work(serde_json::json!({ "action": "requeue", "work_id": work_id }))
            .await?;
        assert_record_ok(&raw, "work") // sak483-i / sak487-i / sak492-h
    }

    /// Alias for terminate-restart / requeue (`sak480-i` / `sak492-h`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn terminate_restart_work(&self, work_id: &str) -> Result<Value, SdkError> {
        self.requeue_work(work_id).await // sak484-i / sak485-i / sak492-h
    }

    /// Queued work count via filtered list (`sak481-i`; payload `session_id` first).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / list assert.
    pub async fn queue_depth(&self, session_id: Option<&str>) -> Result<Value, SdkError> {
        let raw = self
            .list_work_filtered(serde_json::json!({ "status": "queued", "limit": 200 }))
            .await?;
        let items = raw
            .get("work")
            .and_then(|w| w.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(serde_json::json!({
            "queued": queue_depth_for_session(&items, session_id),
            "session_id": session_id,
            "via": "broker",
            "status": "ok",
        }))
    }

    /// Session nodes + queue depth via broker lists (`sak482-i` / `sak485-i` / `sak494-i`).
    ///
    /// Node-list failure returns [`SdkError`]. Queue-list failure after nodes succeed returns
    /// `broker_miss` + `degraded` with nodes preserved — never `via=broker` with `queue_depth=0`.
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / node list assert.
    pub async fn session_compute_status(
        &self,
        session_id: Option<&str>,
        feature: Option<&str>,
    ) -> Result<Value, SdkError> {
        let mut node_filters = serde_json::Map::new();
        if let Some(sid) = session_id {
            node_filters.insert("session_id".into(), Value::String(sid.into()));
        }
        let nodes_raw = self
            .list_nodes_filtered(Value::Object(node_filters))
            .await?;
        let nodes: Vec<Value> = nodes_raw
            .get("nodes")
            .and_then(|n| n.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let obj = item.as_object()?;
                        let label = obj.get("label").cloned().unwrap_or(Value::Null);
                        let mut capabilities = serde_json::Map::new();
                        if let Some(caps) = obj.get("caps").and_then(|c| c.as_array()) {
                            for c in caps {
                                if let Some(s) = c.as_str() {
                                    capabilities.insert(s.into(), Value::Bool(true));
                                }
                            }
                        }
                        Some(serde_json::json!({
                            "node_id": node_id_from_broker_record(item),
                            "display_name": label,
                            "host_label": label,
                            "status": "online",
                            "capabilities": capabilities,
                            "session_id": obj.get("session_id").cloned().unwrap_or(Value::Null),
                            "via": "broker",
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let queued = match self
            .list_work_filtered(serde_json::json!({ "status": "queued", "limit": 200 }))
            .await
        {
            Ok(work_raw) => {
                let items = work_raw
                    .get("work")
                    .and_then(|w| w.as_array())
                    .cloned()
                    .unwrap_or_default();
                queue_depth_for_session(&items, session_id)
            }
            Err(exc) => {
                return Ok(broker_session_queue_miss(&exc, nodes, session_id, feature));
            }
        };

        let mut out = serde_json::Map::new();
        out.insert("nodes".into(), Value::Array(nodes));
        out.insert("queue_depth".into(), serde_json::json!(queued));
        out.insert("via".into(), Value::String("broker".into()));
        out.insert("status".into(), Value::String("ok".into()));
        if let Some(sid) = session_id {
            out.insert("session_id".into(), Value::String(sid.into()));
        }
        if let Some(f) = feature {
            out.insert("feature".into(), Value::String(f.into()));
        }
        Ok(Value::Object(out))
    }

    /// `POST /v1/sak/compute/work` enqueue (`sak431-h` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn enqueue_work(&self, kind: &str, payload: Value) -> Result<Value, SdkError> {
        let raw = self
            .compute_work(serde_json::json!({
                "action": "enqueue",
                "kind": kind,
                "payload": payload
            }))
            .await?;
        assert_record_ok(&raw, "work") // sak483-i / sak487-i / sak492-h
    }

    /// `POST /v1/sak/compute/work` claim (`sak432-f` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / claim normalize.
    pub async fn claim_work(&self, node_id: &str) -> Result<Value, SdkError> {
        let raw = self
            .post_json(
                "/v1/sak/compute/work",
                serde_json::json!({ "action": "claim", "node_id": node_id }),
            )
            .await?;
        normalize_claim_work_response(&raw)
    }

    /// `POST /v1/sak/compute/work` complete (`sak432-f` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn complete_work(
        &self,
        work_id: &str,
        node_id: &str,
        result: Value,
    ) -> Result<Value, SdkError> {
        let raw = self
            .compute_work(serde_json::json!({
                "action": "complete",
                "work_id": work_id,
                "node_id": node_id,
                "result": result
            }))
            .await?;
        assert_record_ok(&raw, "work") // sak483-i / sak487-i / sak492-h
    }

    /// `POST /v1/sak/compute/work` get (`sak433-e` / `sak444-f`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / record assert.
    pub async fn get_work(&self, work_id: &str) -> Result<Value, SdkError> {
        let raw = self
            .compute_work(serde_json::json!({ "action": "get", "work_id": work_id }))
            .await?;
        assert_record_ok(&raw, "work") // sak483-i / sak487-i / sak492-h
    }

    /// `POST /v1/sak/compute/work` filtered list (`sak432-f` / `sak443-h`).
    ///
    /// # Errors
    /// Transport / non-success / JSON parse / list assert.
    pub async fn list_work_filtered(&self, body: Value) -> Result<Value, SdkError> {
        let mut payload = body;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("action".into(), Value::String("list".into()));
        }
        let raw = self.post_json("/v1/sak/compute/work", payload).await?;
        assert_list_ok(&raw, "work")
    }

    async fn get_json(&self, path: &str) -> Result<Value, SdkError> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| SdkError::Http(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| SdkError::Http(e.to_string()))?;
        if !status.is_success() {
            return Err(SdkError::Http(format!("{status}: {body}")));
        }
        serde_json::from_str(&body).map_err(|e| SdkError::Schema(e.to_string()))
    }

    async fn post_json(&self, path: &str, body: Value) -> Result<Value, SdkError> {
        let url = format!("{}{path}", self.base);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| SdkError::Http(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| SdkError::Http(e.to_string()))?;
        if !status.is_success() {
            return Err(SdkError::Http(format!("{status}: {text}")));
        }
        serde_json::from_str(&text).map_err(|e| SdkError::Schema(e.to_string()))
    }
}

/// Session id from broker work unit payload (`sak481-i`).
#[must_use]
pub fn work_session_id(work: &Value) -> Option<&str> {
    if let Some(payload) = work.get("payload").and_then(|p| p.as_object()) {
        if let Some(sid) = payload.get("session_id").and_then(|v| v.as_str()) {
            return Some(sid);
        }
    }
    work.get("session_id").and_then(|v| v.as_str())
}

/// Extract node id from HTTP/MCP node records (`sak482-i`).
#[must_use]
pub fn node_id_from_broker_record(node: &Value) -> String {
    node.get("node_id")
        .or_else(|| node.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned()
}

fn queue_miss_error_message(exc: &SdkError) -> String {
    let err = match exc {
        SdkError::Schema(s) | SdkError::Http(s) => s.clone(),
        SdkError::Broker(c) => c.to_string(),
    };
    if let Some(rest) = err.strip_prefix("broker_miss:") {
        let trimmed = rest.trim_start();
        if let Some((_feat, msg)) = trimmed.split_once(':') {
            return msg.trim().to_string();
        }
        return trimmed.to_string();
    }
    err
}

/// Nodes-ok + queue-fail → `broker_miss`/degraded (never `via=broker` success) (`sak494-i`).
#[must_use]
pub fn broker_session_queue_miss(
    exc: &SdkError,
    nodes: Vec<Value>,
    session_id: Option<&str>,
    feature: Option<&str>,
) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("nodes".into(), Value::Array(nodes));
    out.insert("queue_depth".into(), serde_json::json!(0));
    out.insert("via".into(), Value::String("broker_miss".into()));
    out.insert("status".into(), Value::String("degraded".into()));
    out.insert("error".into(), Value::String(queue_miss_error_message(exc)));
    if let Some(sid) = session_id {
        out.insert("session_id".into(), Value::String(sid.into()));
    }
    if let Some(f) = feature {
        out.insert("feature".into(), Value::String(f.into()));
    }
    Value::Object(out)
}

/// Count queued work; optionally filter by `session_id` (`sak481-i`).
#[must_use]
pub fn queue_depth_for_session(items: &[Value], session_id: Option<&str>) -> usize {
    match session_id {
        None => items.len(),
        Some(sid) => items
            .iter()
            .filter(|w| work_session_id(w).is_some_and(|s| s == sid))
            .count(),
    }
}

const BROKER_MEMORY_ONLY: &str = "broker_memory_only";
const BROKER_SANDBOX_ONLY: &str = "broker_sandbox_only";
const BROKER_TOOLS_ONLY: &str = "broker_tools_only";
const BROKER_RESEARCH_ONLY: &str = "broker_research_only";
const BROKER_EGRESS_ONLY: &str = "broker_egress_only";
const BROKER_LLM_UNAVAILABLE: &str = "broker_llm_unavailable";

fn is_feature_domain_miss(raw: &Value, domain_code: &str, keywords: &[&str]) -> bool {
    let Some(obj) = raw.as_object() else {
        return false;
    };
    if obj.get("code").and_then(Value::as_str) == Some(domain_code) {
        return true;
    }
    if is_compute_miss(raw) {
        return true;
    }
    if let Some(feat) = obj.get("feature").and_then(Value::as_str) {
        let low = feat.to_lowercase();
        if keywords.iter().any(|kw| low.contains(kw)) {
            if obj.get("via").and_then(Value::as_str) == Some("broker_miss") {
                return true;
            }
            if let Some(err) = obj.get("error") {
                if !err.is_null() {
                    let s = err.to_string();
                    if !s.is_empty() && s != "null" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn assert_domain_ok(raw: &Value, is_miss: fn(&Value) -> bool) -> Result<Value, SdkError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| SdkError::Schema("broker_miss: non-object response".into()))?;
    if is_miss(raw) {
        return Err(SdkError::Schema(format!(
            "broker_miss: {}",
            compute_miss_message(obj)
        )));
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            return Err(SdkError::Schema(format!("broker_miss: {err}")));
        }
    }
    Ok(raw.clone())
}

/// True when body is a structured memory peel miss (`sak480-g` / `sak493-i` / `sak495-g`).
#[must_use]
pub fn is_memory_miss(raw: &Value) -> bool {
    is_feature_domain_miss(raw, BROKER_MEMORY_ONLY, &["memory"])
}

/// True when body is a structured sandbox peel miss (`sak496-i`).
#[must_use]
pub fn is_sandbox_miss(raw: &Value) -> bool {
    is_feature_domain_miss(raw, BROKER_SANDBOX_ONLY, &["sandbox"])
}

/// True when body is a structured tools peel miss (`sak496-i`).
#[must_use]
pub fn is_tools_miss(raw: &Value) -> bool {
    is_feature_domain_miss(raw, BROKER_TOOLS_ONLY, &["tools", "shell"])
}

/// True when body is a structured research peel miss (`sak496-i`).
#[must_use]
pub fn is_research_miss(raw: &Value) -> bool {
    is_feature_domain_miss(raw, BROKER_RESEARCH_ONLY, &["research"])
}

/// True when body is a structured egress peel miss (`sak496-i`).
#[must_use]
pub fn is_egress_miss(raw: &Value) -> bool {
    is_feature_domain_miss(raw, BROKER_EGRESS_ONLY, &["egress"])
}

/// True when body is a structured LLM peel miss (`sak496-i`).
#[must_use]
pub fn is_llm_miss(raw: &Value) -> bool {
    is_feature_domain_miss(raw, BROKER_LLM_UNAVAILABLE, &["llm"])
}

/// True when body is a structured compute peel miss (`sak483-i`).
#[must_use]
pub fn is_compute_miss(raw: &Value) -> bool {
    let Some(obj) = raw.as_object() else {
        return false;
    };
    if obj.get("via").and_then(Value::as_str) == Some("broker_miss") {
        return true;
    }
    if obj.get("status").and_then(Value::as_str) == Some("degraded") {
        return true;
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            let s = err.to_string();
            if !s.is_empty() && s != "null" {
                return true;
            }
        }
    }
    false
}

fn compute_miss_message(obj: &serde_json::Map<String, Value>) -> String {
    obj.get("error")
        .or_else(|| obj.get("feature"))
        .or_else(|| obj.get("via"))
        .map(|v| v.to_string())
        .unwrap_or_else(|| "miss".into())
}

/// Empty queue → `{ "work": null, "via": "broker" }`; hard miss → [`SdkError::Schema`] (`sak440-h` / `sak488-i` / `sak490-i` / `sak492-h` claim unchanged).
///
/// `via=broker_miss` always errors — even when `error` mentions an empty queue.
/// # Errors
/// Non-object / hard error / missing work record.
pub fn normalize_claim_work_response(raw: &Value) -> Result<Value, SdkError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| SdkError::Schema("broker_miss: claim: non-object response".into()))?;
    if obj.get("via").and_then(Value::as_str) == Some("broker_miss")
        || obj.get("status").and_then(Value::as_str) == Some("degraded")
    {
        return Err(SdkError::Schema(format!(
            // sak488-i
            "broker_miss: claim: {}",
            compute_miss_message(obj)
        )));
    }
    let work = obj.get("work");
    let err = obj.get("error");
    let err_str = err.map(|v| v.to_string()).unwrap_or_default();
    let low = err_str.to_lowercase();
    let empty_poll = work.map_or(true, Value::is_null)
        && (err.is_none()
            || err.map_or(false, Value::is_null)
            || low.contains("empty")
            || low.contains("no work"));
    if empty_poll {
        return Ok(serde_json::json!({ "work": null, "via": "broker" }));
    }
    if is_compute_miss(raw) {
        return Err(SdkError::Schema(format!(
            "broker_miss: claim: {}",
            compute_miss_message(obj)
        )));
    }
    if err.is_some() && !err.map_or(true, Value::is_null) {
        return Err(SdkError::Schema(format!("broker_miss: claim: {err_str}")));
    }
    if work.map_or(false, Value::is_object) {
        return Ok(raw.clone());
    }
    if obj.get("id").is_some() && obj.get("action").is_none() {
        return Ok(raw.clone());
    }
    Err(SdkError::Schema(
        "broker_miss: claim: missing work record".into(),
    ))
}

/// List assert: error or non-list key raises (`sak441-f` / `sak488-i` / `sak490-i` / `sak491-j`).
///
/// Empty list `[]` is success; null/missing list or peel miss is an error.
/// # Errors
/// Non-object / error field / missing list key.
pub fn assert_list_ok(raw: &Value, list_key: &str) -> Result<Value, SdkError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| SdkError::Schema(format!("broker_miss: non-object for {list_key}")))?;
    if is_compute_miss(raw) {
        // sak488-i
        return Err(SdkError::Schema(format!(
            "broker_miss: {}",
            compute_miss_message(obj)
        )));
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            return Err(SdkError::Schema(format!("broker_miss: {err}")));
        }
    }
    match obj.get(list_key) {
        Some(Value::Array(_)) => Ok(raw.clone()),
        _ => Err(SdkError::Schema(format!(
            "broker_miss: missing or non-list key {list_key}"
        ))),
    }
}

/// Single-record assert for write/get helpers (`sak444-f` / `sak487-i` / `sak491-j` / `sak492-h`).
///
/// Null/missing record is hard miss (no claim-style empty poll).
/// # Errors
/// Non-object / peel miss (`via=broker_miss`) / error field / missing nested or top-level record.
pub fn assert_record_ok(raw: &Value, record_key: &str) -> Result<Value, SdkError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| SdkError::Schema(format!("broker_miss: non-object for {record_key}")))?;
    if is_compute_miss(raw) {
        // sak487-i / sak492-h
        return Err(SdkError::Schema(format!(
            "broker_miss: {}",
            compute_miss_message(obj)
        )));
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            return Err(SdkError::Schema(format!("broker_miss: {err}")));
        }
    }
    if obj.get(record_key).map_or(false, Value::is_object) {
        return Ok(raw.clone());
    }
    if record_key == "work" && obj.get("id").is_some() && obj.get("action").is_none() {
        return Ok(raw.clone());
    }
    if record_key == "node"
        && (obj.get("id").is_some() || obj.get("node_id").is_some())
        && obj.get("nodes").is_none()
    {
        return Ok(raw.clone());
    }
    if record_key == "module" && obj.get("id").is_some() && obj.get("modules").is_none() {
        return Ok(raw.clone());
    }
    Err(SdkError::Schema(format!(
        "broker_miss: missing {record_key} record"
    )))
}

/// Memory search assert: error or non-list hits raises; empty `[]` ok (`sak495-g`).
///
/// # Errors
/// Non-object / peel miss / error field / missing list key.
pub fn assert_memory_ok(raw: &Value, list_key: &str) -> Result<Value, SdkError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| SdkError::Schema(format!("broker_miss: non-object for {list_key}")))?;
    if is_memory_miss(raw) {
        return Err(SdkError::Schema(format!(
            "broker_miss: {}",
            compute_miss_message(obj)
        )));
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            return Err(SdkError::Schema(format!("broker_miss: {err}")));
        }
    }
    match obj.get(list_key) {
        Some(Value::Array(_)) => Ok(raw.clone()),
        _ => Err(SdkError::Schema(format!(
            "broker_miss: missing or non-list key {list_key}"
        ))),
    }
}

/// Domain assert: peel miss or error dict raises (`sak496-i` / `sak498-g`).
///
/// # Errors
/// Non-object / peel miss / error field.
pub fn assert_sandbox_ok(raw: &Value) -> Result<Value, SdkError> {
    assert_domain_ok(raw, is_sandbox_miss)
}

/// Domain assert: peel miss or error dict raises (`sak496-i` / `sak498-g`).
///
/// # Errors
/// Non-object / peel miss / error field.
pub fn assert_tools_ok(raw: &Value) -> Result<Value, SdkError> {
    assert_domain_ok(raw, is_tools_miss)
}

/// Domain assert: peel miss or error dict raises (`sak496-i` / `sak498-g`).
///
/// # Errors
/// Non-object / peel miss / error field.
pub fn assert_research_ok(raw: &Value) -> Result<Value, SdkError> {
    assert_domain_ok(raw, is_research_miss)
}

/// Domain assert: peel miss or error dict raises (`sak496-i` / `sak498-g`).
///
/// # Errors
/// Non-object / peel miss / error field.
pub fn assert_egress_ok(raw: &Value) -> Result<Value, SdkError> {
    assert_domain_ok(raw, is_egress_miss)
}

/// Domain assert: peel miss or error dict raises (`sak496-i` / `sak498-g`).
///
/// # Errors
/// Non-object / peel miss / error field.
pub fn assert_llm_ok(raw: &Value) -> Result<Value, SdkError> {
    assert_domain_ok(raw, is_llm_miss)
}

/// Capacity/health assert: error dict raises; otherwise accept body (`sak446-e` / `sak485-i` / `sak486-i` / `sak493-h`).
///
/// Empty object `{}` is success; peel miss / error field is hard miss.
///
/// # Errors
/// Non-object / error field / peel miss (`via=broker_miss`).
pub fn assert_capacity_ok(raw: &Value) -> Result<Value, SdkError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| SdkError::Schema("broker_miss: non-object for capacity".into()))?;
    if is_compute_miss(raw) {
        return Err(SdkError::Schema(format!(
            "broker_miss: {}",
            compute_miss_message(obj)
        )));
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            return Err(SdkError::Schema(format!("broker_miss: {err}")));
        }
    }
    Ok(raw.clone())
}

/// Raw compute POST assert: error raises except claim empty-polls (`sak447-g`).
///
/// # Errors
/// Error field on non-claim actions.
pub fn assert_raw_compute_post(raw: &Value, body: &Value) -> Result<Value, SdkError> {
    let action = body
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if action == "claim" {
        return Ok(raw.clone());
    }
    let Some(obj) = raw.as_object() else {
        return Ok(raw.clone());
    };
    if is_compute_miss(raw) {
        return Err(SdkError::Schema(format!(
            "broker_miss: {}",
            compute_miss_message(obj)
        )));
    }
    if let Some(err) = obj.get("error") {
        if !err.is_null() {
            return Err(SdkError::Schema(format!("broker_miss: {err}")));
        }
    }
    Ok(raw.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn normalize_claim_empty_vs_miss() {
        let empty = normalize_claim_work_response(&json!({"work": null, "error": "queue empty"}))
            .expect("empty");
        assert_eq!(empty["via"], "broker");
        assert!(empty["work"].is_null());
        assert!(normalize_claim_work_response(&json!({"work": null, "error": "down"})).is_err());
        // sak488-i: via=broker_miss never soft-polls, even with empty-queue error text
        assert!(normalize_claim_work_response(
            &json!({"via": "broker_miss", "work": null, "error": "queue empty"})
        )
        .is_err());
    }

    #[test]
    fn assert_record_ok_write_path_empty_vs_miss() {
        // sak492-h: enqueue/complete/get/requeue — record ok; null/missing/broker_miss hard miss
        assert!(assert_record_ok(&json!({"work": {"id": "w1"}}), "work").is_ok());
        assert!(assert_record_ok(&json!({"id": "w1"}), "work").is_ok());
        assert!(assert_record_ok(&json!({"work": null}), "work").is_err());
        assert!(assert_record_ok(&json!({"work": null, "error": "queue empty"}), "work").is_err());
        assert!(assert_record_ok(&json!({}), "work").is_err());
        assert!(assert_record_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "enqueue"}),
            "work"
        )
        .is_err());
        assert!(assert_record_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "complete"}),
            "work"
        )
        .is_err());
    }

    #[test]
    fn assert_record_ok_get_requeue_miss_vs_record() {
        // sak490-i: get/requeue — record ok; null/missing/broker_miss are hard misses (no soft empty)
        assert!(assert_record_ok(&json!({"work": {"id": "w1"}}), "work").is_ok());
        assert!(assert_record_ok(&json!({"id": "w1"}), "work").is_ok());
        assert!(assert_record_ok(&json!({"work": null}), "work").is_err());
        assert!(assert_record_ok(&json!({}), "work").is_err());
        assert!(assert_record_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "get"}),
            "work"
        )
        .is_err());
        assert!(assert_record_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "requeue"}),
            "work"
        )
        .is_err());
    }

    #[test]
    fn assert_list_ok_empty_vs_null() {
        assert!(assert_list_ok(&json!({"nodes": []}), "nodes").is_ok());
        assert!(assert_list_ok(&json!({"work": []}), "work").is_ok());
        assert!(assert_list_ok(&json!({"nodes": null}), "nodes").is_err());
        assert!(assert_list_ok(&json!({"error": "x", "nodes": []}), "nodes").is_err());
        assert!(assert_list_ok(
            // sak488-i / sak490-i
            &json!({"via": "broker_miss", "work": [], "status": "degraded"}),
            "work"
        )
        .is_err());
        assert!(assert_list_ok(
            // sak491-j
            &json!({"via": "broker_miss", "nodes": [], "status": "degraded"}),
            "nodes"
        )
        .is_err());
    }

    #[test]
    fn assert_record_ok_nested_and_top_level() {
        assert!(assert_record_ok(&json!({"work": {"id": "w1"}}), "work").is_ok());
        assert!(assert_record_ok(&json!({"id": "w1"}), "work").is_ok());
        assert!(assert_record_ok(&json!({"error": "down"}), "work").is_err());
        assert!(assert_record_ok(
            // sak487-i
            &json!({"via": "broker_miss", "status": "degraded", "feature": "enqueue"}),
            "work"
        )
        .is_err());
        assert!(assert_record_ok(&json!({"node": {"id": "n1"}}), "node").is_ok());
        assert!(assert_record_ok(&json!({"nodes": []}), "node").is_err());
        assert!(assert_record_ok(
            // sak484-i / sak491-j
            &json!({"via": "broker_miss", "status": "degraded", "feature": "register"}),
            "node"
        )
        .is_err());
        assert!(assert_record_ok(
            // sak491-j
            &json!({"via": "broker_miss", "status": "degraded", "feature": "heartbeat"}),
            "node"
        )
        .is_err());
    }

    #[test]
    fn assert_list_ok_rejects_module_miss() {
        assert!(assert_list_ok(
            // sak484-i
            &json!({"via": "broker_miss", "status": "degraded", "feature": "list_modules"}),
            "modules"
        )
        .is_err());
    }

    #[test]
    fn assert_capacity_ok_empty_vs_miss() {
        // sak493-h: health/capacity — empty {} ok; error / via=broker_miss hard miss
        assert!(assert_capacity_ok(&json!({})).is_ok());
        assert!(assert_capacity_ok(&json!({"ok": true})).is_ok());
        assert!(assert_capacity_ok(&json!({"snapshot": {"total_ram_mb": 1}})).is_ok());
        assert!(assert_capacity_ok(&json!({"error": "down"})).is_err());
        assert!(assert_capacity_ok(
            // sak485-i / sak493-h
            &json!({"via": "broker_miss", "status": "degraded", "feature": "health"}),
        )
        .is_err());
        assert!(assert_capacity_ok(
            // sak493-h
            &json!({"via": "broker_miss", "status": "degraded", "feature": "capacity"}),
        )
        .is_err());
    }

    #[test]
    fn assert_list_ok_modules_empty_vs_miss() {
        // sak493-h: list_modules — [] ok; null/missing key / broker_miss hard miss
        assert!(assert_list_ok(&json!({"modules": []}), "modules").is_ok());
        assert!(assert_list_ok(&json!({"modules": null}), "modules").is_err());
        assert!(assert_list_ok(&json!({}), "modules").is_err());
        assert!(assert_list_ok(
            // sak484-i / sak493-h
            &json!({"via": "broker_miss", "status": "degraded", "feature": "list_modules"}),
            "modules",
        )
        .is_err());
        assert!(assert_list_ok(
            &json!({"via": "broker_miss", "modules": [], "status": "degraded"}),
            "modules",
        )
        .is_err());
    }

    #[test]
    fn assert_record_ok_module_empty_vs_miss() {
        // sak493-h: get_module — record ok; null/missing / broker_miss hard miss
        assert!(assert_record_ok(&json!({"module": {"id": "m1"}}), "module").is_ok());
        assert!(assert_record_ok(&json!({"id": "m1"}), "module").is_ok());
        assert!(assert_record_ok(&json!({"module": null}), "module").is_err());
        assert!(assert_record_ok(&json!({}), "module").is_err());
        assert!(assert_record_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "get_module"}),
            "module",
        )
        .is_err());
    }

    #[test]
    fn queue_depth_for_session_payload_first() {
        let items = vec![
            json!({"payload": {"session_id": "s1"}}),
            json!({"session_id": "s2"}),
            json!({"payload": {"session_id": "s2"}, "session_id": "s1"}),
        ];
        assert_eq!(queue_depth_for_session(&items, None), 3);
        assert_eq!(queue_depth_for_session(&items, Some("s1")), 1);
        assert_eq!(queue_depth_for_session(&items, Some("s2")), 2);
    }

    #[test]
    fn node_id_from_broker_record_prefers_node_id() {
        assert_eq!(
            node_id_from_broker_record(&json!({"node_id": "n1", "id": "n2"})),
            "n1"
        );
        assert_eq!(node_id_from_broker_record(&json!({"id": "n2"})), "n2");
    }

    #[test]
    fn is_compute_miss_detects_via_and_degraded() {
        assert!(is_compute_miss(&json!({"via": "broker_miss"})));
        assert!(is_compute_miss(&json!({"status": "degraded"})));
        assert!(is_compute_miss(&json!({"error": "down"})));
        assert!(!is_compute_miss(&json!({"work": {"id": "w1"}})));
    }

    #[test]
    fn is_memory_miss_detects_memory_feature_and_broker_only() {
        assert!(is_memory_miss(&json!({"code": "broker_memory_only"})));
        assert!(is_memory_miss(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "fleet_memory_search", "hits": []}),
        ));
        assert!(is_memory_miss(
            &json!({"feature": "fleet_memory_search", "error": "down", "hits": []}),
        ));
        assert!(!is_memory_miss(&json!({"hits": [], "via": "broker"})));
    }

    #[test]
    fn domain_miss_detectors_sak496_i() {
        assert!(is_sandbox_miss(&json!({"code": "broker_sandbox_only"})));
        assert!(is_sandbox_miss(
            &json!({"via": "broker_miss", "feature": "sandbox_exec", "error": "down"}),
        ));
        assert!(!is_sandbox_miss(&json!({"stdout": "ok", "via": "broker"})));

        assert!(is_tools_miss(&json!({"code": "broker_tools_only"})));
        assert!(is_tools_miss(
            &json!({"via": "broker_miss", "feature": "shell", "error": "down"}),
        ));
        assert!(!is_tools_miss(&json!({"stdout": "ok", "via": "broker"})));

        assert!(is_research_miss(&json!({"code": "broker_research_only"})));
        assert!(is_research_miss(
            &json!({"via": "broker_miss", "feature": "research_fetch", "error": "down"}),
        ));

        assert!(is_egress_miss(&json!({"code": "broker_egress_only"})));
        assert!(is_egress_miss(
            &json!({"via": "broker_miss", "feature": "egress_audit", "error": "down"}),
        ));

        assert!(is_llm_miss(&json!({"code": "broker_llm_unavailable"})));
        assert!(is_llm_miss(
            &json!({"via": "broker_miss", "feature": "llm", "error": "down"}),
        ));
        assert!(!is_llm_miss(&json!({"content": "hi", "via": "broker"})));
    }

    #[test]
    fn domain_assert_ok_sak496_i() {
        assert!(assert_sandbox_ok(&json!({"stdout": "ok"})).is_ok());
        assert!(assert_tools_ok(&json!({"stdout": "ok"})).is_ok());
        assert!(assert_research_ok(&json!({"body": "html"})).is_ok());
        assert!(assert_egress_ok(&json!({"allowed": true})).is_ok());
        assert!(assert_llm_ok(&json!({"content": "hi"})).is_ok());
        assert!(
            assert_sandbox_ok(&json!({"via": "broker_miss", "feature": "sandbox_exec"})).is_err()
        );
        assert!(assert_llm_ok(&json!({"code": "broker_llm_unavailable"})).is_err());
    }

    #[test]
    fn assert_memory_ok_empty_vs_miss() {
        assert!(assert_memory_ok(&json!({"hits": []}), "hits").is_ok());
        assert!(
            assert_memory_ok(&json!({"hits": [{"id": "m1"}], "via": "broker"}), "hits").is_ok()
        );
        assert!(assert_memory_ok(&json!({"hits": null}), "hits").is_err());
        assert!(assert_memory_ok(&json!({}), "hits").is_err());
        assert!(assert_memory_ok(
            &json!({"via": "broker_miss", "status": "degraded", "feature": "fleet_memory_search", "hits": []}),
            "hits",
        )
        .is_err());
        assert!(assert_memory_ok(
            &json!({"error": "down", "feature": "fleet_memory_search", "hits": []}),
            "hits",
        )
        .is_err());
    }

    #[tokio::test]
    async fn node_path_empty_vs_miss() {
        // sak491-j: list_nodes / list_nodes_filtered / register / heartbeat HTTP matrix
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"nodes": []})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"nodes": null})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "list",
                "nodes": []
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"nodes": [], "action": "list"})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"nodes": null, "action": "list"})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "list",
                "nodes": []
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"node": {"id": "n1", "label": "w1"}})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"node": null})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "register"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "node": {"id": "n1", "label": "w1"},
                "action": "heartbeat"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "heartbeat"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = SakClient::new(server.uri());
        assert_eq!(client.list_nodes().await.unwrap()["nodes"], json!([]));
        assert!(client.list_nodes().await.is_err());
        assert!(client.list_nodes().await.is_err());

        assert_eq!(
            client
                .list_nodes_filtered(json!({"session_id": "s1"}))
                .await
                .unwrap()["nodes"],
            json!([])
        );
        assert!(client
            .list_nodes_filtered(json!({"session_id": "s1"}))
            .await
            .is_err());
        assert!(client
            .list_nodes_filtered(json!({"session_id": "s1"}))
            .await
            .is_err());

        assert_eq!(
            client
                .register_node(json!({"action": "register", "label": "w1"}))
                .await
                .unwrap()["node"]["id"],
            "n1"
        );
        assert!(client
            .register_node(json!({"action": "register", "label": "w1"}))
            .await
            .is_err());
        assert!(client
            .register_node(json!({"action": "register", "label": "w1"}))
            .await
            .is_err());

        assert_eq!(
            client
                .heartbeat_node(json!({"action": "heartbeat", "node_id": "n1"}))
                .await
                .unwrap()["action"],
            "heartbeat"
        );
        assert!(client
            .heartbeat_node(json!({"action": "heartbeat", "node_id": "n1"}))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn work_write_empty_vs_miss() {
        // sak492-h: enqueue/complete/get/requeue/terminate_restart HTTP matrix; claim empty poll unchanged
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "work": {"id": "w1", "status": "queued"},
                "action": "enqueue"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"work": null, "action": "enqueue"})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "enqueue"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "work": {"id": "w1", "status": "done"},
                "action": "complete"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"work": null, "action": "complete"})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "complete"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "work": {"id": "w1", "status": "queued"}
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"work": null})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "get"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "work": {"id": "w1", "status": "queued"},
                "action": "requeue"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "requeue"
            })))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "work": null,
                "error": "queue empty"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "work": null,
                "error": "queue empty"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = SakClient::new(server.uri());

        assert_eq!(
            client.enqueue_work("echo", json!({})).await.unwrap()["work"]["id"],
            "w1"
        );
        assert!(client.enqueue_work("echo", json!({})).await.is_err());
        assert!(client.enqueue_work("echo", json!({})).await.is_err());

        assert_eq!(
            client.complete_work("w1", "n1", json!({})).await.unwrap()["action"],
            "complete"
        );
        assert!(client.complete_work("w1", "n1", json!({})).await.is_err());
        assert!(client.complete_work("w1", "n1", json!({})).await.is_err());

        assert_eq!(client.get_work("w1").await.unwrap()["work"]["id"], "w1");
        assert!(client.get_work("w1").await.is_err());
        assert!(client.get_work("w1").await.is_err());

        assert_eq!(
            client.requeue_work("w1").await.unwrap()["action"],
            "requeue"
        );
        assert!(client.requeue_work("w1").await.is_err());
        assert!(client.terminate_restart_work("w1").await.is_err());

        let empty = client.claim_work("n1").await.unwrap();
        assert_eq!(empty["via"], "broker");
        assert!(empty["work"].is_null());
        assert!(client.claim_work("n1").await.is_err());
    }

    #[tokio::test]
    async fn readiness_empty_vs_miss() {
        // sak493-h: health/capacity empty {} ok; list_modules [] ok; get_module record ok; null/missing/broker_miss hard miss
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "health"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"modules": []})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"modules": null})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "list_modules",
                "modules": []
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules/demo"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"module": {"id": "demo"}})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules/demo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"module": null})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules/demo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "get_module"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/capacity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/capacity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "via": "broker_miss",
                "status": "degraded",
                "feature": "capacity"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = SakClient::new(server.uri());
        assert!(client.health().await.unwrap().is_object());
        assert!(client.health().await.is_err());

        assert_eq!(client.list_modules().await.unwrap()["modules"], json!([]));
        assert!(client.list_modules().await.is_err());
        assert!(client.list_modules().await.is_err());

        assert_eq!(
            client.get_module("demo").await.unwrap()["module"]["id"],
            "demo"
        );
        assert!(client.get_module("demo").await.is_err());
        assert!(client.get_module("demo").await.is_err());

        assert!(client.capacity().await.unwrap().is_object());
        assert!(client.capacity().await.is_err());
    }

    #[test]
    fn broker_session_queue_miss_strips_feature_prefix() {
        let nodes = vec![json!({"node_id": "n1", "via": "broker"})];
        let exc = SdkError::Schema("broker_miss: list_work_filtered: work down".into());
        let out = broker_session_queue_miss(&exc, nodes.clone(), Some("s1"), Some("fleet_mesh"));
        assert_eq!(out["via"], "broker_miss");
        assert_eq!(out["status"], "degraded");
        assert_eq!(out["queue_depth"], 0);
        assert_eq!(out["nodes"], json!(nodes));
        assert_eq!(out["error"], "work down");
        assert_eq!(out["session_id"], "s1");
        assert_eq!(out["feature"], "fleet_mesh");
    }

    #[tokio::test]
    async fn session_compute_status_nodes_ok_queue_fail_degraded() {
        let server = MockServer::start().await;
        let nid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/nodes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes": [{"id": nid, "label": "n1", "caps": []}],
                "action": "list"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "error": "work down",
                "work": []
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = SakClient::new(server.uri());
        let out = client
            .session_compute_status(Some("s1"), Some("fleet_mesh"))
            .await
            .expect("degraded ok");
        assert_eq!(out["via"], "broker_miss");
        assert_eq!(out["status"], "degraded");
        assert_eq!(out["queue_depth"], 0);
        assert_eq!(out["nodes"].as_array().map(|a| a.len()), Some(1));
        assert_eq!(out["nodes"][0]["node_id"], nid);
        assert!(out["error"].as_str().unwrap_or("").contains("work down"));
    }

    #[tokio::test]
    async fn list_work_filtered_session_id_empty_vs_miss() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "work": [],
                "action": "list"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "error": "work down",
                "work": []
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = SakClient::new(server.uri());
        let empty = client
            .list_work_filtered(json!({"status": "queued", "session_id": "s1"}))
            .await
            .expect("empty ok");
        assert_eq!(empty["work"], json!([]));
        assert!(client
            .list_work_filtered(json!({"status": "queued", "session_id": "s1"}))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn health_and_modules() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/modules"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"modules": []})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/capacity"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"snapshot": {"total_ram_mb": 1}})),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"work": []})))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/sak/compute/work"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"work": [], "action": "list"})),
            )
            .mount(&server)
            .await;

        let client = SakClient::new(server.uri());
        assert_eq!(client.health().await.unwrap()["ok"], true);
        assert!(client.list_modules().await.unwrap()["modules"].is_array());
        assert!(client.capacity().await.unwrap()["snapshot"].is_object());
        assert!(client.list_work().await.unwrap()["work"].is_array());
        let listed = client
            .list_work_filtered(json!({"action": "list", "run_id": "r1"}))
            .await
            .unwrap();
        assert_eq!(listed["action"], "list");
    }
}
