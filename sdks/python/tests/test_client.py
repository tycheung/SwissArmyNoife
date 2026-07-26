"""Unit tests for SwissArmyNoife Python SDK (no live server)."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import httpx
import pytest

from swissarmynoife import SakClient


def test_chat_completions_posts() -> None:
    """sak547-b: chat_completions posts OpenAI facade body."""
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {
        "object": "chat.completion",
        "choices": [{"message": {"role": "assistant", "content": "echo:hi"}}],
    }

    with patch.object(httpx.Client, "post", return_value=mock_resp) as post:
        with SakClient("http://example.test") as sak:
            out = sak.chat_completions(
                {
                    "binding_id": "00000000-0000-0000-0000-000000000001",
                    "model": "echo",
                    "messages": [{"role": "user", "content": "hi"}],
                }
            )
        assert out["object"] == "chat.completion"
        post.assert_called_once()
        args, kwargs = post.call_args
        assert args[0] == "http://example.test/v1/chat/completions"
        assert kwargs["json"]["model"] == "echo"


def test_base_url_strips_trailing_slash() -> None:
    with SakClient("http://127.0.0.1:8787/") as sak:
        assert sak.base_url == "http://127.0.0.1:8787"


def test_health_calls_endpoint() -> None:
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {"ok": True}

    with patch.object(httpx.Client, "get", return_value=mock_resp) as get:
        with SakClient("http://example.test") as sak:
            assert sak.health() == {"ok": True}
        get.assert_called_once_with("http://example.test/health")


@pytest.mark.parametrize(
    ("method", "path", "body"),
    [
        ("list_modules", "/v1/sak/modules", {"modules": []}),
        ("list_work", "/v1/sak/compute/work", {"work": []}),
        ("list_nodes", "/v1/sak/compute/nodes", {"nodes": []}),
        ("capacity", "/v1/sak/capacity", {"snapshot": {"total_ram_mb": 1}}),
    ],
)
def test_list_helpers(method: str, path: str, body: dict) -> None:
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = body

    with patch.object(httpx.Client, "get", return_value=mock_resp) as get:
        with SakClient("http://example.test") as sak:
            out = getattr(sak, method)()
        assert out == body
        get.assert_called_once_with(f"http://example.test{path}")


def test_get_module_encodes_path() -> None:
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {"id": "demo"}

    with patch.object(httpx.Client, "get", return_value=mock_resp) as get:
        with SakClient("http://example.test") as sak:
            assert sak.get_module("demo") == {"id": "demo"}
        get.assert_called_once_with("http://example.test/v1/sak/modules/demo")


def test_register_node_posts() -> None:
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {"node": {"label": "w1"}, "action": "register"}

    with patch.object(httpx.Client, "post", return_value=mock_resp) as post:
        with SakClient("http://example.test") as sak:
            out = sak.register_node("w1", caps=["echo"], session_id="s1")
        assert out["action"] == "register"
        post.assert_called_once()
        args, kwargs = post.call_args
        assert args[0] == "http://example.test/v1/sak/compute/nodes"
        assert kwargs["json"]["action"] == "register"
        assert kwargs["json"]["session_id"] == "s1"


def test_requeue_work_posts() -> None:
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {"work": {"status": "queued"}, "action": "requeue"}

    with patch.object(httpx.Client, "post", return_value=mock_resp) as post:
        with SakClient("http://example.test") as sak:
            out = sak.requeue_work("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        assert out["action"] == "requeue"
        args, kwargs = post.call_args
        assert args[0] == "http://example.test/v1/sak/compute/work"
        assert kwargs["json"]["action"] == "requeue"


def test_enqueue_work_posts() -> None:
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {"work": {"status": "queued"}, "action": "enqueue"}

    with patch.object(httpx.Client, "post", return_value=mock_resp) as post:
        with SakClient("http://example.test") as sak:
            out = sak.enqueue_work("echo", {"n": 1})
        assert out["action"] == "enqueue"
        args, kwargs = post.call_args
        assert args[0] == "http://example.test/v1/sak/compute/work"
        assert kwargs["json"]["action"] == "enqueue"
        assert kwargs["json"]["kind"] == "echo"


def test_normalize_claim_work_response() -> None:
    """sak488-i: empty poll vs via=broker_miss matrix."""
    empty = SakClient.normalize_claim_work_response(
        {"work": None, "error": "queue empty"}
    )
    assert empty == {"work": None, "via": "broker"}
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.normalize_claim_work_response({"work": None, "error": "down"})
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.normalize_claim_work_response(
            {"via": "broker_miss", "work": None, "status": "degraded"}
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.normalize_claim_work_response(
            {"via": "broker_miss", "work": None, "error": "queue empty"}
        )


def test_assert_list_ok() -> None:
    """sak488-i / sak490-i / sak491-j: empty list ok; null/miss raises."""
    assert SakClient.assert_list_ok({"nodes": []}, list_key="nodes")["nodes"] == []
    assert SakClient.assert_list_ok({"work": []}, list_key="work")["work"] == []
    with pytest.raises(RuntimeError, match="non-list"):
        SakClient.assert_list_ok({"nodes": None}, list_key="nodes")
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_list_ok(
            {"via": "broker_miss", "work": [], "status": "degraded"},
            list_key="work",
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_list_ok(
            {"via": "broker_miss", "nodes": [], "status": "degraded"},
            list_key="nodes",
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_list_ok({"error": "x", "work": []}, list_key="work")


def test_compute_write_path_rejects_broker_miss() -> None:
    """sak483-i: enqueue/get/complete/requeue reject via=broker_miss."""
    miss = {"via": "broker_miss", "status": "degraded", "feature": "enqueue"}
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = miss

    with patch.object(httpx.Client, "post", return_value=mock_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.enqueue_work("echo", {"n": 1})
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.get_work("w1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.complete_work("w1", "n1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.requeue_work("w1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.terminate_restart_work("w1")


def test_list_work_filtered_empty_vs_miss() -> None:
    """sak490-i: HTTP list — empty [] ok; null key / broker_miss hard miss."""
    empty_resp = MagicMock()
    empty_resp.raise_for_status = MagicMock()
    empty_resp.json.return_value = {"work": [], "action": "list"}

    null_resp = MagicMock()
    null_resp.raise_for_status = MagicMock()
    null_resp.json.return_value = {"work": None, "action": "list"}

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "list",
        "work": [],
    }

    with patch.object(httpx.Client, "post", return_value=empty_resp):
        with SakClient("http://example.test") as sak:
            out = sak.list_work_filtered(status="queued")
        assert out["work"] == []

    with patch.object(httpx.Client, "post", return_value=null_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="non-list"):
                sak.list_work_filtered(status="queued")

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.list_work_filtered(status="queued")


def test_list_work_filtered_session_id_empty_vs_miss() -> None:
    """sak494-i: filtered list with session_id — empty [] ok; null/miss hard."""
    empty_resp = MagicMock()
    empty_resp.raise_for_status = MagicMock()
    empty_resp.json.return_value = {"work": [], "action": "list"}

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {"error": "work down", "work": []}

    with patch.object(httpx.Client, "post", return_value=empty_resp):
        with SakClient("http://example.test") as sak:
            out = sak.list_work_filtered(status="queued", session_id="s1")
        assert out["work"] == []

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.list_work_filtered(status="queued", session_id="s1")


def test_get_work_record_vs_miss() -> None:
    """sak490-i: HTTP get — work record ok; null/missing/broker_miss hard miss."""
    ok_resp = MagicMock()
    ok_resp.raise_for_status = MagicMock()
    ok_resp.json.return_value = {"work": {"id": "w1", "status": "queued"}}

    null_resp = MagicMock()
    null_resp.raise_for_status = MagicMock()
    null_resp.json.return_value = {"work": None}

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "get",
    }

    with patch.object(httpx.Client, "post", return_value=ok_resp):
        with SakClient("http://example.test") as sak:
            out = sak.get_work("w1")
        assert out["work"]["id"] == "w1"

    with patch.object(httpx.Client, "post", return_value=null_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="missing work record"):
                sak.get_work("w1")

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.get_work("w1")


def test_requeue_work_success_vs_miss() -> None:
    """sak490-i: HTTP requeue — work record ok; broker_miss hard miss."""
    ok_resp = MagicMock()
    ok_resp.raise_for_status = MagicMock()
    ok_resp.json.return_value = {
        "work": {"id": "w1", "status": "queued"},
        "action": "requeue",
    }

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "requeue",
    }

    with patch.object(httpx.Client, "post", return_value=ok_resp):
        with SakClient("http://example.test") as sak:
            out = sak.requeue_work("w1")
        assert out["action"] == "requeue"
        assert out["work"]["status"] == "queued"

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.requeue_work("w1")


def test_assert_record_ok_write_path_empty_vs_miss() -> None:
    """sak492-h: enqueue/complete/get/requeue — record ok; null/missing/broker_miss hard miss."""
    assert SakClient.assert_record_ok({"work": {"id": "w1"}}, record_key="work")["work"]["id"] == "w1"
    assert SakClient.assert_record_ok({"id": "w1"}, record_key="work")["id"] == "w1"
    with pytest.raises(RuntimeError, match="missing work record"):
        SakClient.assert_record_ok({"work": None}, record_key="work")
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_record_ok(
            {"work": None, "error": "queue empty"},
            record_key="work",
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_record_ok(
            {"via": "broker_miss", "status": "degraded", "feature": "enqueue"},
            record_key="work",
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_record_ok(
            {"via": "broker_miss", "status": "degraded", "feature": "complete"},
            record_key="work",
        )


def test_work_write_empty_vs_miss() -> None:
    """sak492-h: HTTP enqueue/complete/get/requeue/terminate_restart; claim empty poll unchanged."""
    responses = [
        {"work": {"id": "w1", "status": "queued"}, "action": "enqueue"},
        {"work": None, "action": "enqueue"},
        {"via": "broker_miss", "status": "degraded", "feature": "enqueue"},
        {"work": {"id": "w1", "status": "done"}, "action": "complete"},
        {"work": None, "action": "complete"},
        {"via": "broker_miss", "status": "degraded", "feature": "complete"},
        {"work": {"id": "w1", "status": "queued"}},
        {"work": None},
        {"via": "broker_miss", "status": "degraded", "feature": "get"},
        {"work": {"id": "w1", "status": "queued"}, "action": "requeue"},
        {"via": "broker_miss", "status": "degraded", "feature": "requeue"},
        {"via": "broker_miss", "status": "degraded", "feature": "requeue"},
        {"work": None, "error": "queue empty"},
        {"via": "broker_miss", "work": None, "error": "queue empty"},
    ]
    mock_responses = []
    for body in responses:
        resp = MagicMock()
        resp.raise_for_status = MagicMock()
        resp.json.return_value = body
        mock_responses.append(resp)

    with patch.object(httpx.Client, "post", side_effect=mock_responses):
        with SakClient("http://example.test") as sak:
            out = sak.enqueue_work("echo", {"n": 1})
            assert out["work"]["id"] == "w1"
            with pytest.raises(RuntimeError, match="missing work record"):
                sak.enqueue_work("echo", {"n": 1})
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.enqueue_work("echo", {"n": 1})

            out = sak.complete_work("w1", "n1")
            assert out["action"] == "complete"
            with pytest.raises(RuntimeError, match="missing work record"):
                sak.complete_work("w1", "n1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.complete_work("w1", "n1")

            out = sak.get_work("w1")
            assert out["work"]["id"] == "w1"
            with pytest.raises(RuntimeError, match="missing work record"):
                sak.get_work("w1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.get_work("w1")

            out = sak.requeue_work("w1")
            assert out["action"] == "requeue"
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.requeue_work("w1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.terminate_restart_work("w1")

            empty = sak.claim_work("n1")
            assert empty == {"work": None, "via": "broker"}
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.claim_work("n1")


def test_assert_capacity_ok_empty_vs_miss() -> None:
    """sak493-h: health/capacity — empty {} ok; error / via=broker_miss hard miss."""
    assert SakClient.assert_capacity_ok({}) == {}
    assert SakClient.assert_capacity_ok({"ok": True})["ok"] is True
    assert SakClient.assert_capacity_ok({"snapshot": {"total_ram_mb": 1}})["snapshot"]["total_ram_mb"] == 1
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_capacity_ok({"error": "down"})
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_capacity_ok(
            {"via": "broker_miss", "status": "degraded", "feature": "health"},
            feature="health",
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_capacity_ok(
            {"via": "broker_miss", "status": "degraded", "feature": "capacity"},
            feature="capacity",
        )


def test_assert_list_ok_modules_empty_vs_miss() -> None:
    """sak493-h: list_modules — [] ok; null/missing key / broker_miss hard miss."""
    assert SakClient.assert_list_ok({"modules": []}, list_key="modules")["modules"] == []
    with pytest.raises(RuntimeError, match="non-list"):
        SakClient.assert_list_ok({"modules": None}, list_key="modules")
    with pytest.raises(RuntimeError, match="non-list"):
        SakClient.assert_list_ok({}, list_key="modules")
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_list_ok(
            {"via": "broker_miss", "status": "degraded", "feature": "list_modules"},
            list_key="modules",
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_list_ok(
            {"via": "broker_miss", "modules": [], "status": "degraded"},
            list_key="modules",
        )


def test_assert_record_ok_module_empty_vs_miss() -> None:
    """sak493-h: get_module — record ok; null/missing / broker_miss hard miss."""
    assert SakClient.assert_record_ok({"module": {"id": "m1"}}, record_key="module")["module"]["id"] == "m1"
    assert SakClient.assert_record_ok({"id": "m1"}, record_key="module")["id"] == "m1"
    with pytest.raises(RuntimeError, match="missing module record"):
        SakClient.assert_record_ok({"module": None}, record_key="module")
    with pytest.raises(RuntimeError, match="missing module record"):
        SakClient.assert_record_ok({}, record_key="module")
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_record_ok(
            {"via": "broker_miss", "status": "degraded", "feature": "get_module"},
            record_key="module",
        )


def test_readiness_empty_vs_miss() -> None:
    """sak493-h: HTTP health/capacity/list_modules/get_module empty-vs-miss matrix."""
    responses = [
        {},
        {"via": "broker_miss", "status": "degraded", "feature": "health"},
        {"modules": []},
        {"modules": None},
        {
            "via": "broker_miss",
            "status": "degraded",
            "feature": "list_modules",
            "modules": [],
        },
        {"module": {"id": "demo"}},
        {"module": None},
        {"via": "broker_miss", "status": "degraded", "feature": "get_module"},
        {},
        {"via": "broker_miss", "status": "degraded", "feature": "capacity"},
    ]
    mock_responses = []
    for body in responses:
        resp = MagicMock()
        resp.raise_for_status = MagicMock()
        resp.json.return_value = body
        mock_responses.append(resp)

    with patch.object(httpx.Client, "get", side_effect=mock_responses):
        with SakClient("http://example.test") as sak:
            assert sak.health() == {}
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.health()

            out = sak.list_modules()
            assert out["modules"] == []
            with pytest.raises(RuntimeError, match="non-list"):
                sak.list_modules()
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.list_modules()

            out = sak.get_module("demo")
            assert out["module"]["id"] == "demo"
            with pytest.raises(RuntimeError, match="missing module record"):
                sak.get_module("demo")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.get_module("demo")

            assert sak.capacity() == {}
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.capacity()


def test_health_rejects_via_broker_miss() -> None:
    """sak485-i: health rejects via=broker_miss."""
    miss = {"via": "broker_miss", "status": "degraded", "feature": "health"}
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = miss

    with patch.object(httpx.Client, "get", return_value=mock_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.health()


def test_get_module_rejects_via_broker_miss() -> None:
    """sak485-i: get_module rejects via=broker_miss."""
    miss = {"via": "broker_miss", "status": "degraded", "feature": "get_module"}
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = miss

    with patch.object(httpx.Client, "get", return_value=mock_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.get_module("demo")


def test_list_nodes_empty_vs_miss() -> None:
    """sak491-j: HTTP GET list_nodes — empty [] ok; null key / broker_miss hard miss."""
    empty_resp = MagicMock()
    empty_resp.raise_for_status = MagicMock()
    empty_resp.json.return_value = {"nodes": []}

    null_resp = MagicMock()
    null_resp.raise_for_status = MagicMock()
    null_resp.json.return_value = {"nodes": None}

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "list",
        "nodes": [],
    }

    with patch.object(httpx.Client, "get", return_value=empty_resp):
        with SakClient("http://example.test") as sak:
            out = sak.list_nodes()
        assert out["nodes"] == []

    with patch.object(httpx.Client, "get", return_value=null_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="non-list"):
                sak.list_nodes()

    with patch.object(httpx.Client, "get", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.list_nodes()


def test_list_nodes_filtered_empty_vs_miss() -> None:
    """sak491-j: HTTP POST list_nodes_filtered — empty [] ok; null / broker_miss hard miss."""
    empty_resp = MagicMock()
    empty_resp.raise_for_status = MagicMock()
    empty_resp.json.return_value = {"nodes": [], "action": "list"}

    null_resp = MagicMock()
    null_resp.raise_for_status = MagicMock()
    null_resp.json.return_value = {"nodes": None, "action": "list"}

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "list",
        "nodes": [],
    }

    with patch.object(httpx.Client, "post", return_value=empty_resp):
        with SakClient("http://example.test") as sak:
            out = sak.list_nodes_filtered(session_id="s1")
        assert out["nodes"] == []

    with patch.object(httpx.Client, "post", return_value=null_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="non-list"):
                sak.list_nodes_filtered(session_id="s1")

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.list_nodes_filtered(session_id="s1")


def test_register_node_record_vs_miss() -> None:
    """sak491-j: HTTP register — node record ok; null/missing/broker_miss hard miss."""
    ok_resp = MagicMock()
    ok_resp.raise_for_status = MagicMock()
    ok_resp.json.return_value = {"node": {"id": "n1", "label": "w1"}}

    null_resp = MagicMock()
    null_resp.raise_for_status = MagicMock()
    null_resp.json.return_value = {"node": None}

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "register",
    }

    with patch.object(httpx.Client, "post", return_value=ok_resp):
        with SakClient("http://example.test") as sak:
            out = sak.register_node("w1")
        assert out["node"]["id"] == "n1"

    with patch.object(httpx.Client, "post", return_value=null_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="missing node record"):
                sak.register_node("w1")

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.register_node("w1")


def test_heartbeat_node_record_vs_miss() -> None:
    """sak491-j: HTTP heartbeat — node record ok; broker_miss hard miss."""
    ok_resp = MagicMock()
    ok_resp.raise_for_status = MagicMock()
    ok_resp.json.return_value = {
        "node": {"id": "n1", "label": "w1"},
        "action": "heartbeat",
    }

    miss_resp = MagicMock()
    miss_resp.raise_for_status = MagicMock()
    miss_resp.json.return_value = {
        "via": "broker_miss",
        "status": "degraded",
        "feature": "heartbeat",
    }

    with patch.object(httpx.Client, "post", return_value=ok_resp):
        with SakClient("http://example.test") as sak:
            out = sak.heartbeat_node("n1")
        assert out["action"] == "heartbeat"
        assert out["node"]["id"] == "n1"

    with patch.object(httpx.Client, "post", return_value=miss_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.heartbeat_node("n1")


def test_compute_node_path_rejects_via_broker_miss() -> None:
    """sak484-i: register/heartbeat/list_modules reject via=broker_miss."""
    miss = {"via": "broker_miss", "status": "degraded", "feature": "register"}
    post_resp = MagicMock()
    post_resp.raise_for_status = MagicMock()
    post_resp.json.return_value = miss
    get_resp = MagicMock()
    get_resp.raise_for_status = MagicMock()
    get_resp.json.return_value = miss

    with patch.object(httpx.Client, "post", return_value=post_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.register_node("w1")
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.heartbeat_node("n1")

    with patch.object(httpx.Client, "get", return_value=get_resp):
        with SakClient("http://example.test") as sak:
            with pytest.raises(RuntimeError, match="broker_miss"):
                sak.list_modules()


def test_queue_depth_for_session_payload_first() -> None:
    items = [
        {"payload": {"session_id": "s1"}},
        {"session_id": "s2"},
        {"payload": {"session_id": "s2"}, "session_id": "s1"},
    ]
    assert SakClient.queue_depth_for_session(items, None) == 3
    assert SakClient.queue_depth_for_session(items, "s1") == 1
    assert SakClient.queue_depth_for_session(items, "s2") == 2


def test_session_compute_status_nodes_ok_queue_fail_degraded() -> None:
    """sak494-i: nodes-ok + queue-fail → broker_miss/degraded (nodes preserved)."""
    nid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    nodes_resp = MagicMock()
    nodes_resp.raise_for_status = MagicMock()
    nodes_resp.json.return_value = {
        "nodes": [{"id": nid, "label": "n1", "caps": []}],
        "action": "list",
    }
    work_miss_resp = MagicMock()
    work_miss_resp.raise_for_status = MagicMock()
    work_miss_resp.json.return_value = {"error": "work down", "work": []}

    with patch.object(httpx.Client, "post", side_effect=[nodes_resp, work_miss_resp]):
        with SakClient("http://example.test") as sak:
            out = sak.session_compute_status("s1", feature="fleet_mesh")
    assert out["via"] == "broker_miss"
    assert out["status"] == "degraded"
    assert out["queue_depth"] == 0
    assert len(out["nodes"]) == 1
    assert out["nodes"][0]["node_id"] == nid
    assert "work down" in str(out.get("error") or "")


def test_assert_memory_ok_empty_vs_miss() -> None:
    """sak495-g: memory search — empty hits ok; null/missing/broker_miss hard miss."""
    assert SakClient.assert_memory_ok({"hits": []})["hits"] == []
    assert SakClient.assert_memory_ok({"hits": [{"id": "m1"}], "via": "broker"})["hits"][0]["id"] == "m1"
    with pytest.raises(RuntimeError, match="missing or non-list key 'hits'"):
        SakClient.assert_memory_ok({"hits": None})
    with pytest.raises(RuntimeError, match="missing or non-list key 'hits'"):
        SakClient.assert_memory_ok({})
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_memory_ok(
            {
                "via": "broker_miss",
                "status": "degraded",
                "feature": "fleet_memory_search",
                "hits": [],
            }
        )
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_memory_ok(
            {"error": "down", "feature": "fleet_memory_search", "hits": []},
        )


def test_is_memory_miss() -> None:
    """sak495-g: memory peel miss detector mirrors admin isMemoryMiss."""
    assert SakClient.is_memory_miss({"code": "broker_memory_only"}) is True
    assert SakClient.is_memory_miss(
        {
            "via": "broker_miss",
            "status": "degraded",
            "feature": "fleet_memory_search",
            "hits": [],
        }
    ) is True
    assert SakClient.is_memory_miss(
        {"feature": "fleet_memory_search", "error": "down", "hits": []},
    ) is True
    assert SakClient.is_memory_miss({"hits": [], "via": "broker"}) is False


def test_domain_miss_detectors_sak496_i() -> None:
    """sak496-i: domain peel miss detectors mirror peel_assert."""
    assert SakClient.is_sandbox_miss({"code": "broker_sandbox_only"}) is True
    assert SakClient.is_sandbox_miss(
        {"via": "broker_miss", "feature": "sandbox_exec", "error": "down"},
    ) is True
    assert SakClient.is_sandbox_miss({"stdout": "ok", "via": "broker"}) is False

    assert SakClient.is_tools_miss({"code": "broker_tools_only"}) is True
    assert SakClient.is_tools_miss(
        {"via": "broker_miss", "feature": "shell", "error": "down"},
    ) is True

    assert SakClient.is_research_miss({"code": "broker_research_only"}) is True
    assert SakClient.is_egress_miss({"code": "broker_egress_only"}) is True
    assert SakClient.is_llm_miss({"code": "broker_llm_unavailable"}) is True
    assert SakClient.is_llm_miss({"content": "hi", "via": "broker"}) is False


def test_domain_assert_ok_sak496_i() -> None:
    """sak496-i: domain asserts raise on peel miss."""
    assert SakClient.assert_sandbox_ok({"stdout": "ok"})["stdout"] == "ok"
    assert SakClient.assert_llm_ok({"content": "hi"})["content"] == "hi"
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_sandbox_ok({"via": "broker_miss", "feature": "sandbox_exec"})
    with pytest.raises(RuntimeError, match="broker_miss"):
        SakClient.assert_llm_ok({"code": "broker_llm_unavailable"})


def test_session_compute_status_merges_nodes_and_queue() -> None:
    nid = "11111111-2222-3333-4444-555555555555"
    nodes_resp = MagicMock()
    nodes_resp.raise_for_status = MagicMock()
    nodes_resp.json.return_value = {
        "nodes": [{"id": nid, "label": "n1", "caps": ["mesh"], "session_id": "s1"}],
        "action": "list",
    }
    work_resp = MagicMock()
    work_resp.raise_for_status = MagicMock()
    work_resp.json.return_value = {
        "work": [{"status": "queued", "session_id": "s1"}],
        "action": "list",
    }

    with patch.object(httpx.Client, "post", side_effect=[nodes_resp, work_resp]) as post:
        with SakClient("http://example.test") as sak:
            out = sak.session_compute_status("s1", feature="fleet_mesh")
        assert out["via"] == "broker"
        assert out["feature"] == "fleet_mesh"
        assert out["queue_depth"] == 1
        assert out["nodes"][0]["node_id"] == nid
        assert post.call_count == 2
