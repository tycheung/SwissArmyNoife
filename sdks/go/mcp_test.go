package swissarmynoife_test

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"

	swissarmynoife "github.com/tycheung/swissarmynoife-sdk"
)

func TestMcpPingWithSession(t *testing.T) {
	var sawSession bool
	var n int
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw, _ := io.ReadAll(r.Body)
		var body map[string]any
		_ = json.Unmarshal(raw, &body)
		method, _ := body["method"].(string)
		n++
		switch method {
		case "initialize":
			w.Header().Set("mcp-session-id", "sess-go-1")
			_ = json.NewEncoder(w).Encode(map[string]any{
				"jsonrpc": "2.0",
				"id":      body["id"],
				"result":  map[string]any{"protocolVersion": "2024-11-05"},
			})
		case "notifications/initialized":
			w.WriteHeader(http.StatusAccepted)
		case "tools/call":
			if r.Header.Get("mcp-session-id") != "sess-go-1" {
				t.Fatalf("missing session header")
			}
			sawSession = true
			params := body["params"].(map[string]any)
			if params["name"] != "ping" {
				t.Fatalf("%v", params)
			}
			_ = json.NewEncoder(w).Encode(map[string]any{
				"jsonrpc": "2.0",
				"id":      body["id"],
				"result": map[string]any{
					"content": []any{map[string]any{"type": "text", "text": "pong"}},
				},
			})
		default:
			t.Fatalf("unexpected method %s", method)
		}
	}))
	defer srv.Close()

	m := swissarmynoife.NewMcpClient(srv.URL)
	text, err := m.Ping()
	if err != nil {
		t.Fatal(err)
	}
	if text != "pong" {
		t.Fatalf("got %q", text)
	}
	if m.SessionID() != "sess-go-1" {
		t.Fatalf("session=%q", m.SessionID())
	}
	if !sawSession || n < 3 {
		t.Fatalf("sawSession=%v n=%d", sawSession, n)
	}
}

func TestMcpToolsListNoAutoInit(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw, _ := io.ReadAll(r.Body)
		var body map[string]any
		_ = json.Unmarshal(raw, &body)
		if body["method"] != "tools/list" {
			t.Fatalf("%v", body)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{
			"jsonrpc": "2.0",
			"id":      body["id"],
			"result":  map[string]any{"tools": []any{}},
		})
	}))
	defer srv.Close()

	m := swissarmynoife.NewMcpClient(srv.URL)
	m.AutoInitialize = false
	out, err := m.ToolsList()
	if err != nil {
		t.Fatal(err)
	}
	obj := out.(map[string]any)
	if _, ok := obj["tools"]; !ok {
		t.Fatalf("%v", out)
	}
}

func TestMcpCatalogList(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw, _ := io.ReadAll(r.Body)
		var body map[string]any
		_ = json.Unmarshal(raw, &body)
		method, _ := body["method"].(string)
		switch method {
		case "initialize":
			w.Header().Set("mcp-session-id", "s2")
			_ = json.NewEncoder(w).Encode(map[string]any{"jsonrpc": "2.0", "id": body["id"], "result": map[string]any{}})
		case "notifications/initialized":
			w.WriteHeader(http.StatusAccepted)
		case "tools/call":
			params := body["params"].(map[string]any)
			if params["name"] != "catalog_list" {
				t.Fatalf("%v", params)
			}
			_ = json.NewEncoder(w).Encode(map[string]any{
				"jsonrpc": "2.0",
				"id":      body["id"],
				"result":  map[string]any{"offers": []any{}},
			})
		default:
			t.Fatalf("method=%s body=%s", method, string(raw))
		}
	}))
	defer srv.Close()

	m := swissarmynoife.NewMcpClient(srv.URL)
	out, err := m.CatalogList()
	if err != nil {
		t.Fatal(err)
	}
	obj, ok := out.(map[string]any)
	if !ok {
		t.Fatalf("%v", out)
	}
	if _, ok := obj["offers"]; !ok {
		t.Fatalf("%v", out)
	}
}
