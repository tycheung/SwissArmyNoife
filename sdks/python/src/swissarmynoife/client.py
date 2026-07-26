"""HTTP admin client mirroring the Rust `sdk` crate.

Rust ``sdk`` is HTTP-only. ``SakMcpClient`` (``mcp.py``) adds MCP ``compute_work`` /
``claim_work``; work-write empty-vs-miss asserts live here on the HTTP surface
(``sak490-i`` / ``sak492-h``); node-path list/register/heartbeat parity (``sak491-j``).
"""

from __future__ import annotations

from typing import Any

import httpx


class SakClient:
    """Thin wrapper over SwissArmyNoife `http-admin` endpoints."""

    def __init__(self, base_url: str, *, timeout: float = 30.0) -> None:
        self.base_url = base_url.rstrip("/")
        self._http = httpx.Client(timeout=timeout)

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> SakClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()

    def health(self) -> Any:
        """GET /health; raise on peel error dict (`sak449-i` / `sak485-i` / `sak486-i`)."""
        return self.assert_capacity_ok(  # sak485-i / sak486-i / sak493-h
            self._get_json("/health"),
            feature="health",
        )

    def list_modules(self) -> Any:
        return self.assert_list_ok(  # sak484-i / sak485-i / sak493-h
            self._get_json("/v1/sak/modules"),
            list_key="modules",
            feature="list_modules",
        )

    def get_module(self, module_id: str) -> Any:
        return self.assert_record_ok(  # sak485-i / sak493-h
            self._get_json(f"/v1/sak/modules/{module_id}"),
            record_key="module",
            feature="get_module",
        )

    def capacity(self) -> Any:
        return self.assert_capacity_ok(  # sak486-i / sak493-h
            self._get_json("/v1/sak/capacity"),
            feature="capacity",
        )

    def chat_completions(self, body: dict[str, Any]) -> Any:
        """POST /v1/chat/completions OpenAI-shaped facade (`sak547-a`)."""
        return self._post_json("/v1/chat/completions", body)

    def list_work(self) -> Any:
        return self.assert_list_ok(
            self._get_json("/v1/sak/compute/work"),
            list_key="work",
            feature="list_work",
        )

    def list_nodes(self) -> Any:
        return self.assert_list_ok(  # sak491-j
            self._get_json("/v1/sak/compute/nodes"),
            list_key="nodes",
            feature="list_nodes",
        )

    def compute_nodes(self, payload: dict[str, Any]) -> Any:
        """POST register/heartbeat/list (`sak427-f` / `sak447-g`)."""
        return self._assert_raw_compute_post(
            self._post_json("/v1/sak/compute/nodes", payload),
            payload,
            feature="compute_nodes",
        )

    def register_node(
        self,
        label: str,
        *,
        caps: list[str] | None = None,
        node_id: str | None = None,
        session_id: str | None = None,
    ) -> Any:
        body: dict[str, Any] = {"action": "register", "label": label}
        if caps is not None:
            body["caps"] = list(caps)
        if node_id:
            body["node_id"] = node_id
        if session_id:
            body["session_id"] = session_id
        return self.assert_record_ok(  # sak484-i / sak487-i / sak491-j
            self.compute_nodes(body),
            record_key="node",
            feature="register_node",
        )

    def heartbeat_node(self, node_id: str) -> Any:
        return self.assert_record_ok(  # sak484-i / sak487-i / sak491-j
            self.compute_nodes({"action": "heartbeat", "node_id": node_id}),
            record_key="node",
            feature="heartbeat_node",
        )

    def compute_work(self, payload: dict[str, Any]) -> Any:
        """POST enqueue/claim/complete/get/list/requeue (`sak429-c` / `sak447-g`)."""
        return self._assert_raw_compute_post(
            self._post_json("/v1/sak/compute/work", payload),
            payload,
            feature="compute_work",
        )

    @staticmethod
    def is_compute_miss(raw: Any) -> bool:
        """True when body is a structured compute peel miss (`sak483-i`)."""
        if not isinstance(raw, dict):
            return False
        if raw.get("via") == "broker_miss":
            return True
        if raw.get("status") == "degraded":
            return True
        if "error" in raw and raw.get("error") is not None:
            err = str(raw.get("error") or "")
            if err:
                return True
        return False

    @staticmethod
    def _feature_domain_miss(
        raw: Any,
        *,
        domain_code: str,
        keywords: tuple[str, ...],
    ) -> bool:
        """Shared domain peel miss detector (`sak496-i`)."""
        if not isinstance(raw, dict):
            return False
        if raw.get("code") == domain_code:
            return True
        if SakClient.is_compute_miss(raw):
            return True
        feat = raw.get("feature")
        if isinstance(feat, str):
            low = feat.lower()
            if any(kw in low for kw in keywords):
                if raw.get("via") == "broker_miss":
                    return True
                err = raw.get("error")
                if err is not None and str(err):
                    return True
        return False

    @staticmethod
    def _assert_domain_ok(
        raw: Any,
        *,
        feature: str,
        is_miss: Any,
    ) -> dict[str, Any]:
        """Domain assert: peel miss or error dict raises (`sak496-i`)."""
        if not isinstance(raw, dict):
            raise RuntimeError(f"broker_miss: {feature}: non-dict response: {raw!r}")
        if is_miss(raw):
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw and raw.get("error") is not None:
            raise RuntimeError(f"broker_miss: {feature}: {raw.get('error')!r}")
        return raw

    @staticmethod
    def is_memory_miss(raw: Any) -> bool:
        """True when body is a structured memory peel miss (`sak480-g` / `sak493-i` / `sak495-g`)."""
        return SakClient._feature_domain_miss(raw, domain_code="broker_memory_only", keywords=("memory",))

    @staticmethod
    def is_sandbox_miss(raw: Any) -> bool:
        """True when body is a structured sandbox peel miss (`sak496-i`)."""
        return SakClient._feature_domain_miss(
            raw, domain_code="broker_sandbox_only", keywords=("sandbox",)
        )

    @staticmethod
    def is_tools_miss(raw: Any) -> bool:
        """True when body is a structured tools peel miss (`sak496-i`)."""
        return SakClient._feature_domain_miss(
            raw, domain_code="broker_tools_only", keywords=("tools", "shell")
        )

    @staticmethod
    def is_research_miss(raw: Any) -> bool:
        """True when body is a structured research peel miss (`sak496-i`)."""
        return SakClient._feature_domain_miss(
            raw, domain_code="broker_research_only", keywords=("research",)
        )

    @staticmethod
    def is_egress_miss(raw: Any) -> bool:
        """True when body is a structured egress peel miss (`sak496-i`)."""
        return SakClient._feature_domain_miss(
            raw, domain_code="broker_egress_only", keywords=("egress",)
        )

    @staticmethod
    def is_llm_miss(raw: Any) -> bool:
        """True when body is a structured LLM peel miss (`sak496-i`)."""
        return SakClient._feature_domain_miss(
            raw, domain_code="broker_llm_unavailable", keywords=("llm",)
        )

    @staticmethod
    def assert_sandbox_ok(raw: Any, *, feature: str = "sandbox_exec") -> dict[str, Any]:
        """Sandbox assert: peel miss or error dict raises (`sak496-i`)."""
        return SakClient._assert_domain_ok(raw, feature=feature, is_miss=SakClient.is_sandbox_miss)

    @staticmethod
    def assert_tools_ok(raw: Any, *, feature: str = "shell") -> dict[str, Any]:
        """Tools assert: peel miss or error dict raises (`sak496-i`)."""
        return SakClient._assert_domain_ok(raw, feature=feature, is_miss=SakClient.is_tools_miss)

    @staticmethod
    def assert_research_ok(raw: Any, *, feature: str = "research_fetch") -> dict[str, Any]:
        """Research assert: peel miss or error dict raises (`sak496-i`)."""
        return SakClient._assert_domain_ok(raw, feature=feature, is_miss=SakClient.is_research_miss)

    @staticmethod
    def assert_egress_ok(raw: Any, *, feature: str = "egress") -> dict[str, Any]:
        """Egress assert: peel miss or error dict raises (`sak496-i`)."""
        return SakClient._assert_domain_ok(raw, feature=feature, is_miss=SakClient.is_egress_miss)

    @staticmethod
    def assert_llm_ok(raw: Any, *, feature: str = "llm") -> dict[str, Any]:
        """LLM assert: peel miss or error dict raises (`sak496-i`)."""
        return SakClient._assert_domain_ok(raw, feature=feature, is_miss=SakClient.is_llm_miss)

    @staticmethod
    def assert_memory_ok(
        raw: Any,
        *,
        feature: str = "memory_search",
        list_key: str = "hits",
    ) -> dict[str, Any]:
        """Memory search assert: peel miss or error dict raises; empty hits ok (`sak495-g`)."""
        if not isinstance(raw, dict):
            raise RuntimeError(f"broker_miss: {feature}: non-dict response: {raw!r}")
        if SakClient.is_memory_miss(raw):  # sak495-g
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw and raw.get("error") is not None:
            raise RuntimeError(f"broker_miss: {feature}: {raw.get('error')!r}")
        val = raw.get(list_key)
        if not isinstance(val, list):
            raise RuntimeError(
                f"broker_miss: {feature}: missing or non-list key {list_key!r}"
            )
        return raw

    @staticmethod
    def _assert_raw_compute_post(
        raw: Any,
        payload: dict[str, Any],
        *,
        feature: str,
    ) -> Any:
        if not isinstance(raw, dict):
            return raw
        if str(payload.get("action") or "") == "claim":
            return raw
        if SakClient.is_compute_miss(raw):  # sak483-i
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw and raw.get("error") is not None:
            raise RuntimeError(f"broker_miss: {feature}: {raw.get('error')!r}")
        return raw

    def requeue_work(self, work_id: str) -> Any:
        return self.assert_record_ok(  # sak483-i / sak487-i / sak492-h
            self.compute_work({"action": "requeue", "work_id": work_id}),
            record_key="work",
            feature="requeue_work",
        )

    def terminate_restart_work(self, work_id: str) -> Any:
        """Alias for requeue / terminate-restart (`sak480-i` / `sak485-i` / `sak492-h`)."""
        return self.requeue_work(work_id)  # sak484-i / sak485-i / sak492-h

    def queue_depth(self, session_id: str | None = None) -> dict[str, Any]:
        """Queued work count via filtered list (`sak481-i`; payload session_id first)."""
        raw = self.list_work_filtered(status="queued", limit=200)
        items = [w for w in (raw.get("work") or []) if isinstance(w, dict)]
        return {
            "queued": self.queue_depth_for_session(items, session_id),
            "session_id": session_id,
            "via": "broker",
            "status": "ok",
        }

    @staticmethod
    def _broker_session_queue_miss(
        exc: BaseException,
        *,
        nodes: list[dict[str, Any]],
        session_id: str | None,
        feature: str | None,
    ) -> dict[str, Any]:
        """Nodes-ok + queue-fail → ``broker_miss``/degraded (never ``via=broker`` success) (`sak494-i`)."""
        err = str(exc)
        if err.startswith("broker_miss:"):
            parts = err.split(":", 2)
            if len(parts) >= 3:
                err = parts[2].strip()
        out: dict[str, Any] = {
            "nodes": nodes,
            "queue_depth": 0,
            "via": "broker_miss",
            "status": "degraded",
            "error": err,
        }
        if session_id is not None:
            out["session_id"] = session_id
        if feature:
            out["feature"] = feature
        return out

    def session_compute_status(
        self,
        session_id: str | None = None,
        *,
        feature: str | None = None,
    ) -> dict[str, Any]:
        """Session nodes + queue depth via broker lists (`sak482-i` / `sak485-i` / `sak494-i`)."""
        filters: dict[str, Any] = {}
        if session_id:
            filters["session_id"] = session_id
        nodes_raw = self.list_nodes_filtered(**filters)
        nodes: list[dict[str, Any]] = []
        for item in nodes_raw.get("nodes") or []:
            if not isinstance(item, dict):
                continue
            caps = item.get("caps") or []
            nodes.append(
                {
                    "node_id": self.node_id_from_broker_record(item),
                    "display_name": item.get("label"),
                    "host_label": item.get("label"),
                    "status": "online",
                    "capabilities": {str(c): True for c in caps if isinstance(c, str)},
                    "session_id": item.get("session_id"),
                    "via": "broker",
                }
            )
        try:
            work_raw = self.list_work_filtered(status="queued", limit=200)
            items = [w for w in (work_raw.get("work") or []) if isinstance(w, dict)]
            queued = self.queue_depth_for_session(items, session_id)
        except Exception as exc:  # noqa: BLE001
            return self._broker_session_queue_miss(
                exc,
                nodes=nodes,
                session_id=session_id,
                feature=feature,
            )
        out: dict[str, Any] = {
            "nodes": nodes,
            "queue_depth": queued,
            "via": "broker",
            "status": "ok",
        }
        if session_id is not None:
            out["session_id"] = session_id
        if feature:
            out["feature"] = feature
        return out

    @staticmethod
    def work_session_id(work: dict[str, Any]) -> str:
        """Session id from broker work unit payload (`sak481-i`)."""
        payload = work.get("payload")
        if isinstance(payload, dict) and payload.get("session_id") is not None:
            return str(payload.get("session_id"))
        if work.get("session_id") is not None:
            return str(work.get("session_id"))
        return ""

    @staticmethod
    def node_id_from_broker_record(node: dict[str, Any]) -> str:
        """Extract node id from HTTP/MCP node records (`sak482-i`)."""
        return str(node.get("node_id") or node.get("id") or "")

    @staticmethod
    def queue_depth_for_session(
        work_items: list[dict[str, Any]],
        session_id: str | None,
    ) -> int:
        """Count queued work; optionally filter by session_id (`sak481-i`)."""
        if not session_id:
            return len(work_items)
        return sum(1 for w in work_items if SakClient.work_session_id(w) == session_id)

    def enqueue_work(self, kind: str, payload: dict[str, Any] | None = None) -> Any:
        """POST ``compute_work`` enqueue (`sak431-h` / `sak445-g`)."""
        return self.assert_record_ok(  # sak483-i / sak487-i / sak492-h
            self.compute_work(
                {"action": "enqueue", "kind": kind, "payload": dict(payload or {})}
            ),
            record_key="work",
            feature="enqueue_work",
        )

    def claim_work(self, node_id: str) -> Any:
        """POST ``compute_work`` claim (`sak432-f` / `sak445-g`)."""
        return self.normalize_claim_work_response(
            self.compute_work({"action": "claim", "node_id": node_id})
        )

    @staticmethod
    def normalize_claim_work_response(raw: Any, *, feature: str = "claim") -> dict[str, Any]:
        """Empty queue → ``work: None``; ``via=broker_miss`` always raises (`sak440-h` / `sak488-i` / `sak490-i` / `sak492-h` claim unchanged)."""
        if not isinstance(raw, dict):
            raise RuntimeError(f"broker_miss: {feature}: non-dict response: {raw!r}")
        if raw.get("via") == "broker_miss" or raw.get("status") == "degraded":  # sak488-i
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        work = raw.get("work")
        err = raw.get("error") if "error" in raw else None
        err_str = str(err or "")
        empty_poll = work is None and (
            err is None
            or "empty" in err_str.lower()
            or "no work" in err_str.lower()
        )
        if empty_poll:
            return {"work": None, "via": "broker"}
        if SakClient.is_compute_miss(raw):  # sak483-i
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw:
            raise RuntimeError(f"broker_miss: {feature}: {err!r}")
        if isinstance(work, dict):
            return raw
        if raw.get("id") is not None and "action" not in raw:
            return raw
        raise RuntimeError(f"broker_miss: {feature}: missing work record")

    @staticmethod
    def assert_list_ok(
        raw: Any,
        *,
        list_key: str = "nodes",
        feature: str = "list",
    ) -> dict[str, Any]:
        """Error or non-list key raises (`sak441-f` / `sak488-i` / `sak490-i` / `sak491-j`).

        Empty list ``[]`` is success; ``null``/missing list or ``via=broker_miss`` is a miss.
        """
        if not isinstance(raw, dict):
            raise RuntimeError(f"broker_miss: {feature}: non-dict response: {raw!r}")
        if SakClient.is_compute_miss(raw):  # sak483-i / sak488-i
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw:
            raise RuntimeError(f"broker_miss: {feature}: {raw.get('error')!r}")
        if not isinstance(raw.get(list_key), list):
            raise RuntimeError(
                f"broker_miss: {feature}: missing or non-list key {list_key!r}"
            )
        return raw

    @staticmethod
    def assert_record_ok(
        raw: Any,
        *,
        record_key: str = "work",
        feature: str = "record",
    ) -> dict[str, Any]:
        """Single-record assert for write/get helpers (`sak445-g` / `sak446-f` / `sak487-i` / `sak491-j` / `sak492-h`).

        Rejects ``via=broker_miss`` before record shape checks. Null/missing record is hard miss
        (no claim-style empty poll).
        """
        if not isinstance(raw, dict):
            raise RuntimeError(f"broker_miss: {feature}: non-dict response: {raw!r}")
        if SakClient.is_compute_miss(raw):  # sak487-i / sak492-h
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw:
            raise RuntimeError(f"broker_miss: {feature}: {raw.get('error')!r}")
        rec = raw.get(record_key)
        if isinstance(rec, dict):
            return raw
        if record_key == "work" and raw.get("id") is not None and "action" not in raw:
            return raw
        if record_key == "node" and (
            raw.get("id") is not None or raw.get("node_id") is not None
        ):
            if "nodes" not in raw:
                return raw
        if record_key == "module" and raw.get("id") is not None and "modules" not in raw:
            return raw
        raise RuntimeError(f"broker_miss: {feature}: missing {record_key} record")

    @staticmethod
    def assert_capacity_ok(raw: Any, *, feature: str = "capacity") -> dict[str, Any]:
        """Capacity/health assert: error dict raises (`sak446-f` / `sak485-i` / `sak486-i` / `sak493-h`).

        Empty dict ``{}`` is success; peel miss / error field is hard miss.
        """
        if not isinstance(raw, dict):
            raise RuntimeError(f"broker_miss: {feature}: non-dict response: {raw!r}")
        if SakClient.is_compute_miss(raw):  # sak485-i / sak486-i
            raise RuntimeError(
                f"broker_miss: {feature}: {raw.get('error') or raw.get('feature') or raw.get('via') or 'miss'!r}"
            )
        if "error" in raw:
            raise RuntimeError(f"broker_miss: {feature}: {raw.get('error')!r}")
        return raw

    def complete_work(
        self,
        work_id: str,
        node_id: str,
        result: dict[str, Any] | None = None,
    ) -> Any:
        """POST ``compute_work`` complete (`sak432-f` / `sak445-g`)."""
        return self.assert_record_ok(  # sak483-i / sak487-i / sak492-h
            self.compute_work(
                {
                    "action": "complete",
                    "work_id": work_id,
                    "node_id": node_id,
                    "result": dict(result or {}),
                }
            ),
            record_key="work",
            feature="complete_work",
        )

    def get_work(self, work_id: str) -> Any:
        """POST ``compute_work`` get (`sak433-e` / `sak445-g`)."""
        return self.assert_record_ok(  # sak483-i / sak487-i / sak492-h
            self.compute_work({"action": "get", "work_id": work_id}),
            record_key="work",
            feature="get_work",
        )

    def list_work_filtered(self, **filters: Any) -> Any:
        """POST ``compute_work`` list (`sak432-f` / `sak443-h` / `sak488-i`)."""
        body: dict[str, Any] = {"action": "list"}
        body.update({k: v for k, v in filters.items() if v is not None})
        return self.assert_list_ok(  # sak488-i / sak490-i
            self.compute_work(body),
            list_key="work",
            feature="list_work_filtered",
        )

    def list_nodes_filtered(self, **filters: Any) -> Any:
        """POST ``compute_nodes`` list (`sak433-e` / `sak443-h`)."""
        body: dict[str, Any] = {"action": "list"}
        body.update({k: v for k, v in filters.items() if v is not None})
        return self.assert_list_ok(  # sak491-j
            self.compute_nodes(body),
            list_key="nodes",
            feature="list_nodes_filtered",
        )

    def _get_json(self, path: str) -> Any:
        resp = self._http.get(f"{self.base_url}{path}")
        resp.raise_for_status()
        return resp.json()

    def _post_json(self, path: str, payload: dict[str, Any]) -> Any:
        resp = self._http.post(f"{self.base_url}{path}", json=payload)
        resp.raise_for_status()
        return resp.json()
