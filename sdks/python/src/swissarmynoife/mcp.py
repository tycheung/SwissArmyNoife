"""Streamable HTTP MCP client (`sak322-d` / `sak329-b` / `sak489-i` / `sak490-i`).

MCP exposes ``compute_work`` + ``claim_work`` only. List/get/requeue empty-vs-miss
asserts are on ``SakClient`` (HTTP); Rust ``sdk`` is HTTP-only as well.
"""

from __future__ import annotations

import json
from typing import Any

import httpx

from swissarmynoife.client import SakClient

_DEFAULT_MCP_URL = "http://127.0.0.1:8080/mcp"
_MCP_PROTOCOL_VERSION = "2024-11-05"
_SESSION_HEADER = "mcp-session-id"
_MCP_ACCEPT = "application/json, text/event-stream"


def _unwrap_mcp_compute_payload(result: Any) -> Any:
    """Unwrap MCP tool content + InvokeResp into HTTP-shaped compute JSON (``sak489-i``)."""
    payload: Any = result
    if isinstance(payload, dict):
        content = payload.get("content")
        if isinstance(content, list):
            for item in content:
                if isinstance(item, dict):
                    text = item.get("text")
                    if isinstance(text, str):
                        try:
                            payload = json.loads(text)
                        except json.JSONDecodeError:
                            payload = text
                        break
    if isinstance(payload, dict):
        status = payload.get("status")
        if status == "ok" and "result" in payload:
            return payload.get("result")
        if status == "error":
            return {"work": None, "error": payload.get("message") or "invoke error"}
    return payload


def _extract_ping_text(result: Any) -> str:
    if isinstance(result, str):
        return result
    if isinstance(result, dict):
        content = result.get("content")
        if isinstance(content, list):
            for item in content:
                if isinstance(item, dict):
                    text = item.get("text")
                    if isinstance(text, str):
                        return text
    return str(result)


def _session_id_from_body(body: Any) -> str | None:
    if not isinstance(body, dict):
        return None
    for key in ("sessionId", "session_id", "mcp-session-id"):
        val = body.get(key)
        if isinstance(val, str) and val.strip():
            return val.strip()
    result = body.get("result")
    if isinstance(result, dict):
        for key in ("sessionId", "session_id", "mcp-session-id"):
            val = result.get(key)
            if isinstance(val, str) and val.strip():
                return val.strip()
    return None


class SakMcpClient:
    """MCP client over Streamable HTTP with optional ``initialize`` session (``sak329-b``)."""

    def __init__(
        self,
        base_url: str = _DEFAULT_MCP_URL,
        *,
        token: str | None = None,
        timeout: float = 30.0,
        client: httpx.Client | None = None,
        auto_initialize: bool = True,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self._token = token
        self._timeout = timeout
        self._client = client
        self._rpc_id = 0
        self._session_id: str | None = None
        self._initialized = False
        self._auto_initialize = auto_initialize

    @property
    def session_id(self) -> str | None:
        return self._session_id

    def _auth_headers(self) -> dict[str, str]:
        headers = {
            "Content-Type": "application/json",
            "Accept": _MCP_ACCEPT,
        }
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"
        if self._session_id:
            headers[_SESSION_HEADER] = self._session_id
        return headers

    def _post(
        self,
        payload: dict[str, Any],
        *,
        notification: bool = False,
    ) -> httpx.Response:
        headers = self._auth_headers()
        if self._client is not None:
            response = self._client.post(
                self.base_url,
                json=payload,
                headers=headers,
                timeout=self._timeout,
            )
            if notification and response.status_code in (200, 202):
                return response
            response.raise_for_status()
            return response
        with httpx.Client(timeout=self._timeout) as owned:
            response = owned.post(self.base_url, json=payload, headers=headers)
            if notification and response.status_code in (200, 202):
                return response
            response.raise_for_status()
            return response

    def _rpc(self, method: str, params: dict[str, Any] | None = None) -> Any:
        self._rpc_id += 1
        payload = {
            "jsonrpc": "2.0",
            "id": self._rpc_id,
            "method": method,
            "params": params or {},
        }
        response = self._post(payload)
        header_sid = response.headers.get(_SESSION_HEADER)
        if header_sid and header_sid.strip() and not self._session_id:
            self._session_id = header_sid.strip()
        body = response.json()
        if isinstance(body, dict):
            if not self._session_id:
                sid = _session_id_from_body(body)
                if sid:
                    self._session_id = sid
            if "error" in body:
                err = body["error"]
                message = err.get("message", err) if isinstance(err, dict) else err
                raise RuntimeError(f"MCP {method} failed: {message}")
            return body.get("result", body)
        return body

    def initialize(self) -> Any:
        """Post MCP ``initialize`` + ``notifications/initialized``; capture session id."""
        result = self._rpc(
            "initialize",
            {
                "protocolVersion": _MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "swissarmynoife", "version": "0.1.0"},
            },
        )
        self._post(
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            notification=True,
        )
        self._initialized = True
        return result

    def ensure_session(self) -> None:
        if not self._auto_initialize or self._initialized:
            return
        self.initialize()

    def _tools_call(self, name: str, arguments: dict[str, Any] | None = None) -> Any:
        self.ensure_session()
        return self._rpc(
            "tools/call",
            {"name": name, "arguments": arguments or {}},
        )

    def ping(self) -> str:
        result = self._tools_call("ping")
        return _extract_ping_text(result)

    def tools_list(self) -> Any:
        """MCP ``tools/list`` (``sak322-e``)."""
        self.ensure_session()
        return self._rpc("tools/list")

    def catalog_list(self) -> Any:
        """MCP ``catalog_list`` via ``tools/call`` (``sak322-e``)."""
        return self._tools_call("catalog_list")

    def compute_work(self, payload: dict[str, Any]) -> Any:
        """MCP ``compute_work`` — raw invoke body; claim skips hard assert (``sak489-i``)."""
        raw = _unwrap_mcp_compute_payload(
            self._tools_call("compute_work", payload),
        )
        return SakClient._assert_raw_compute_post(  # sak489-i
            raw,
            payload,
            feature="compute_work",
        )

    def claim_work(self, node_id: str, *, binding_id: str) -> Any:
        """MCP ``compute_work`` claim + shared empty-vs-miss normalize (``sak489-i``)."""
        return SakClient.normalize_claim_work_response(  # sak488-i / sak489-i / sak490-i
            self.compute_work(
                {"binding_id": binding_id, "action": "claim", "node_id": node_id}
            ),
            feature="claim_work",
        )

    def _domain_tool_raw(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        raw = self._tools_call(name, arguments)
        return raw if isinstance(raw, dict) else {"result": raw}

    def sandbox_exec(self, argv: list[str], cwd: str = ".") -> dict[str, Any]:
        """MCP ``sandbox_exec`` with domain peel assert (``sak498-g``)."""
        return SakClient.assert_sandbox_ok(
            self._domain_tool_raw("sandbox_exec", {"argv": argv, "cwd": cwd}),
            feature="sandbox_exec",
        )

    def shell_exec(self, argv: list[str], cwd: str = ".") -> dict[str, Any]:
        """MCP ``shell_exec`` with domain peel assert (``sak498-g``)."""
        return SakClient.assert_tools_ok(
            self._domain_tool_raw("shell_exec", {"argv": argv, "cwd": cwd}),
            feature="shell_exec",
        )

    def research_fetch(self, url: str) -> dict[str, Any]:
        """MCP ``research_fetch`` with domain peel assert (``sak498-g``)."""
        return SakClient.assert_research_ok(
            self._domain_tool_raw("research_fetch", {"url": url}),
            feature="research_fetch",
        )

    def egress_check(self, url: str) -> dict[str, Any]:
        """MCP ``egress_check`` with domain peel assert (``sak498-g``)."""
        return SakClient.assert_egress_ok(
            self._domain_tool_raw("egress_check", {"url": url}),
            feature="egress_check",
        )

    def llm_chat(
        self,
        messages: list[dict[str, Any]],
        *,
        model: str | None = None,
    ) -> dict[str, Any]:
        """MCP ``llm_chat`` with domain peel assert (``sak498-g``)."""
        arguments: dict[str, Any] = {"messages": messages}
        if model is not None:
            arguments["model"] = model
        return SakClient.assert_llm_ok(
            self._domain_tool_raw("llm_chat", arguments),
            feature="llm_chat",
        )

    def memory_index(
        self,
        binding_id: str,
        documents: list[dict[str, str]],
        *,
        scope_key: str | None = None,
    ) -> dict[str, Any]:
        """MCP ``memory_index`` with domain peel assert (``sak499-i``)."""
        arguments: dict[str, Any] = {
            "binding_id": binding_id,
            "documents": documents,
        }
        if scope_key is not None:
            arguments["scope_key"] = scope_key
        return SakClient._assert_domain_ok(
            self._domain_tool_raw("memory_index", arguments),
            feature="memory_index",
            is_miss=SakClient.is_memory_miss,
        )
