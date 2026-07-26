"""Unit tests for SakMcpClient (mocked httpx)."""

from __future__ import annotations

from unittest.mock import MagicMock

import httpx
import pytest

from swissarmynoife.mcp import SakMcpClient


def _tool_client(mock_client: httpx.Client, **kwargs: object) -> SakMcpClient:
    return SakMcpClient(
        "http://127.0.0.1:8080/mcp",
        client=mock_client,
        auto_initialize=False,
        **kwargs,  # type: ignore[arg-type]
    )


def test_initialize_captures_session_and_sends_initialized() -> None:
    mock_client = MagicMock(spec=httpx.Client)

    def _post(url: str, **kwargs: object) -> MagicMock:
        method = (kwargs.get("json") or {}).get("method")  # type: ignore[union-attr]
        resp = MagicMock()
        resp.raise_for_status = MagicMock()
        if method == "initialize":
            resp.headers = {"mcp-session-id": "sess-abc"}
            resp.json.return_value = {
                "jsonrpc": "2.0",
                "id": 1,
                "result": {"protocolVersion": "2024-11-05"},
            }
            resp.status_code = 200
        elif method == "notifications/initialized":
            resp.headers = {}
            resp.status_code = 202
            resp.json.return_value = {}
        else:
            resp.headers = {}
            resp.status_code = 200
            resp.json.return_value = {
                "jsonrpc": "2.0",
                "id": 2,
                "result": {"content": [{"type": "text", "text": "pong"}]},
            }
        return resp

    mock_client.post.side_effect = _post
    client = SakMcpClient("http://127.0.0.1:8080/mcp", client=mock_client)
    client.initialize()
    assert client.session_id == "sess-abc"
    assert client.ping() == "pong"
    methods = [c.kwargs["json"]["method"] for c in mock_client.post.call_args_list]
    assert methods[:2] == ["initialize", "notifications/initialized"]
    assert methods[2] == "tools/call"
    assert mock_client.post.call_args_list[2].kwargs["headers"]["mcp-session-id"] == "sess-abc"


def test_ping_auto_initializes_once() -> None:
    mock_client = MagicMock(spec=httpx.Client)
    init_count = {"n": 0}

    def _post(url: str, **kwargs: object) -> MagicMock:
        method = (kwargs.get("json") or {}).get("method")  # type: ignore[union-attr]
        resp = MagicMock()
        resp.raise_for_status = MagicMock()
        if method == "initialize":
            init_count["n"] += 1
            resp.headers = {"mcp-session-id": "s1"}
            resp.json.return_value = {"jsonrpc": "2.0", "id": 1, "result": {}}
            resp.status_code = 200
        elif method == "notifications/initialized":
            resp.headers = {}
            resp.status_code = 202
            resp.json.return_value = {}
        else:
            resp.headers = {}
            resp.status_code = 200
            resp.json.return_value = {
                "jsonrpc": "2.0",
                "id": 2,
                "result": {"content": [{"type": "text", "text": "pong"}]},
            }
        return resp

    mock_client.post.side_effect = _post
    client = SakMcpClient("http://127.0.0.1:8080/mcp", client=mock_client)
    assert client.ping() == "pong"
    assert client.ping() == "pong"
    assert init_count["n"] == 1
    assert client.session_id == "s1"


def test_ping_posts_tools_call_and_returns_text() -> None:
    mock_client = MagicMock(spec=httpx.Client)
    mock_response = MagicMock()
    mock_response.headers = {}
    mock_response.json.return_value = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"content": [{"type": "text", "text": "pong"}]},
    }
    mock_response.raise_for_status = MagicMock()
    mock_client.post.return_value = mock_response

    client = _tool_client(mock_client, token="tok")
    out = client.ping()

    assert out == "pong"
    call = mock_client.post.call_args
    assert call.args[0] == "http://127.0.0.1:8080/mcp"
    assert call.kwargs["json"]["method"] == "tools/call"
    assert call.kwargs["json"]["params"] == {"name": "ping", "arguments": {}}
    assert call.kwargs["headers"]["Authorization"] == "Bearer tok"


def test_bind_and_invoke_post_tools_call() -> None:
    """sak329-c: bind / invoke / provision thin wrappers."""
    mock_client = MagicMock(spec=httpx.Client)
    mock_response = MagicMock()
    mock_response.headers = {}
    mock_response.json.return_value = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"binding_id": "b1"},
    }
    mock_response.raise_for_status = MagicMock()
    mock_client.post.return_value = mock_response

    client = _tool_client(mock_client)
    client.bind("llm.chat", principal="local")
    assert mock_client.post.call_args.kwargs["json"]["params"]["name"] == "bind"
    assert (
        mock_client.post.call_args.kwargs["json"]["params"]["arguments"]["offer_id"]
        == "llm.chat"
    )
    client.invoke("b1", {"messages": []}, offer="llm.chat")
    assert mock_client.post.call_args.kwargs["json"]["params"]["name"] == "invoke"
    client.provision("llm.chat", idempotency_key="idem-1")
    assert mock_client.post.call_args.kwargs["json"]["params"]["name"] == "provision"


def test_default_mcp_url_strips_trailing_slash() -> None:
    client = SakMcpClient("http://127.0.0.1:8080/mcp/")
    assert client.base_url == "http://127.0.0.1:8080/mcp"


def test_tools_list_posts_tools_list() -> None:
    mock_client = MagicMock(spec=httpx.Client)
    mock_response = MagicMock()
    mock_response.json.return_value = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"tools": [{"name": "ping"}, {"name": "catalog_list"}]},
    }
    mock_response.headers = {}
    mock_response.raise_for_status = MagicMock()
    mock_client.post.return_value = mock_response

    client = _tool_client(mock_client)
    out = client.tools_list()

    assert out == {"tools": [{"name": "ping"}, {"name": "catalog_list"}]}
    call = mock_client.post.call_args
    assert call.kwargs["json"]["method"] == "tools/list"
    assert call.kwargs["json"]["params"] == {}


def test_catalog_list_posts_tools_call() -> None:
    mock_client = MagicMock(spec=httpx.Client)
    mock_response = MagicMock()
    mock_response.json.return_value = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"offers": [{"id": "llm.chat"}]},
    }
    mock_response.headers = {}
    mock_response.raise_for_status = MagicMock()
    mock_client.post.return_value = mock_response

    client = _tool_client(mock_client)
    out = client.catalog_list()

    assert out == {"offers": [{"id": "llm.chat"}]}
    call = mock_client.post.call_args
    assert call.kwargs["json"]["method"] == "tools/call"
    assert call.kwargs["json"]["params"] == {"name": "catalog_list", "arguments": {}}


def test_compute_work_posts_tools_call() -> None:
    """sak489-i: MCP compute_work unwraps InvokeResp and posts tools/call."""
    mock_client = MagicMock(spec=httpx.Client)
    mock_response = MagicMock()
    mock_response.json.return_value = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": '{"status":"ok","result":{"action":"claim","work":{"id":"w1"}}}',
                }
            ]
        },
    }
    mock_response.raise_for_status = MagicMock()
    mock_response.headers = {}
    mock_client.post.return_value = mock_response

    client = _tool_client(mock_client)
    out = client.compute_work(
        {"binding_id": "b1", "action": "claim", "node_id": "n1"},
    )

    assert out == {"action": "claim", "work": {"id": "w1"}}
    call = mock_client.post.call_args
    assert call.kwargs["json"]["method"] == "tools/call"
    assert call.kwargs["json"]["params"]["name"] == "compute_work"
    assert call.kwargs["json"]["params"]["arguments"]["action"] == "claim"


def test_claim_work_normalize_empty_vs_miss() -> None:
    """sak489-i / sak490-i: shared normalize_claim empty-vs-miss matrix via MCP client."""
    from swissarmynoife.client import SakClient

    empty = SakClient.normalize_claim_work_response(
        {"work": None, "error": "no work available"},
        feature="claim_work",
    )
    assert empty == {"work": None, "via": "broker"}
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.normalize_claim_work_response(
            {"via": "broker_miss", "work": None, "error": "queue empty"},
            feature="claim_work",
        )


def test_sak498_g_mcp_domain_assert_helpers() -> None:
    """sak498-g: SakMcpClient domain helpers raise on peel miss."""
    client = SakMcpClient("http://127.0.0.1:8080/mcp")
    client._tools_call = MagicMock(  # type: ignore[method-assign]
        return_value={"via": "broker_miss", "feature": "sandbox_exec", "error": "down"},
    )
    with pytest.raises(RuntimeError, match="broker_miss"):
        client.sandbox_exec(["echo"])

    client._tools_call = MagicMock(  # type: ignore[method-assign]
        return_value={"stdout": "ok", "via": "broker"},
    )
    assert client.sandbox_exec(["echo"])["stdout"] == "ok"


def test_memory_index_posts_tools_call() -> None:
    """sak499-i: MCP memory_index unwraps index body."""
    mock_client = MagicMock(spec=httpx.Client)
    mock_response = MagicMock()
    mock_response.json.return_value = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "rebuilt": True,
            "vector_count": 2,
            "fingerprint": "fp1",
            "backend": "exact",
            "scope_key": "default",
        },
    }
    mock_response.raise_for_status = MagicMock()
    mock_response.headers = {}
    mock_client.post.return_value = mock_response

    client = _tool_client(mock_client)
    out = client.memory_index(
        "b-mem",
        [{"id": "1", "text": "alpha"}, {"id": "2", "text": "beta"}],
    )

    assert out["rebuilt"] is True
    call = mock_client.post.call_args
    assert call.kwargs["json"]["method"] == "tools/call"
    assert call.kwargs["json"]["params"]["name"] == "memory_index"
    assert call.kwargs["json"]["params"]["arguments"]["binding_id"] == "b-mem"


def test_sak499_i_memory_index_raises_on_peel_miss() -> None:
    """sak499-i: memory_index raises on broker_memory_only peel envelope."""
    client = SakMcpClient("http://127.0.0.1:8080/mcp")
    client._tools_call = MagicMock(  # type: ignore[method-assign]
        return_value={
            "code": "broker_memory_only",
            "error": "use SwissArmyNoife memory_index",
        },
    )
    with pytest.raises(RuntimeError, match="broker_miss"):
        client.memory_index("b1", [{"id": "1", "text": "x"}])
