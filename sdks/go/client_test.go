package swissarmynoife_test

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"

	swissarmynoife "github.com/tycheung/swissarmynoife-sdk"
)

func TestNewClientStripsSlash(t *testing.T) {
	c := swissarmynoife.NewClient("http://127.0.0.1:8787/")
	if c.BaseURL != "http://127.0.0.1:8787" {
		t.Fatalf("BaseURL=%q", c.BaseURL)
	}
}

func TestHealth(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/health" {
			t.Fatalf("path=%s", r.URL.Path)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	c := swissarmynoife.NewClient(srv.URL)
	out, err := c.Health()
	if err != nil {
		t.Fatal(err)
	}
	m := out.(map[string]any)
	if m["ok"] != true {
		t.Fatalf("%v", out)
	}
}

func TestListHelpers(t *testing.T) {
	cases := []struct {
		path string
		body map[string]any
		call func(*swissarmynoife.Client) (any, error)
	}{
		{"/v1/sak/modules", map[string]any{"modules": []any{}}, (*swissarmynoife.Client).ListModules},
		{"/v1/sak/compute/work", map[string]any{"work": []any{}}, (*swissarmynoife.Client).ListWork},
		{"/v1/sak/compute/nodes", map[string]any{"nodes": []any{}}, (*swissarmynoife.Client).ListNodes},
		{"/v1/sak/capacity", map[string]any{"snapshot": map[string]any{"total_ram_mb": 1.0}}, (*swissarmynoife.Client).Capacity},
	}
	for _, tc := range cases {
		t.Run(tc.path, func(t *testing.T) {
			srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				if r.URL.Path != tc.path {
					t.Fatalf("got %s", r.URL.Path)
				}
				_ = json.NewEncoder(w).Encode(tc.body)
			}))
			defer srv.Close()
			c := swissarmynoife.NewClient(srv.URL)
			out, err := tc.call(c)
			if err != nil {
				t.Fatal(err)
			}
			if out == nil {
				t.Fatal("nil")
			}
		})
	}
}

func TestEnqueueWork(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/sak/compute/work" || r.Method != http.MethodPost {
			t.Fatalf("%s %s", r.Method, r.URL.Path)
		}
		raw, _ := io.ReadAll(r.Body)
		var body map[string]any
		_ = json.Unmarshal(raw, &body)
		if body["action"] != "enqueue" || body["kind"] != "echo" {
			t.Fatalf("%v", body)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{"action": "enqueue", "work": map[string]any{"status": "queued"}})
	}))
	defer srv.Close()

	c := swissarmynoife.NewClient(srv.URL)
	out, err := c.EnqueueWork("echo", map[string]any{"n": 1})
	if err != nil {
		t.Fatal(err)
	}
	m := out.(map[string]any)
	if m["action"] != "enqueue" {
		t.Fatalf("%v", out)
	}
}

func TestRequeueAndClaim(t *testing.T) {
	var lastAction string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw, _ := io.ReadAll(r.Body)
		var body map[string]any
		_ = json.Unmarshal(raw, &body)
		lastAction = body["action"].(string)
		_ = json.NewEncoder(w).Encode(map[string]any{"action": lastAction, "work": map[string]any{"id": "w1"}})
	}))
	defer srv.Close()
	c := swissarmynoife.NewClient(srv.URL)

	if _, err := c.RequeueWork("w1"); err != nil {
		t.Fatal(err)
	}
	if lastAction != "requeue" {
		t.Fatalf("action=%s", lastAction)
	}
	if _, err := c.ClaimWork("n1"); err != nil {
		t.Fatal(err)
	}
	if lastAction != "claim" {
		t.Fatalf("action=%s", lastAction)
	}
	if _, err := c.CompleteWork("w1", "n1", nil); err != nil {
		t.Fatal(err)
	}
	if lastAction != "complete" {
		t.Fatalf("action=%s", lastAction)
	}
	if _, err := c.GetWork("w1"); err != nil {
		t.Fatal(err)
	}
	if lastAction != "get" {
		t.Fatalf("action=%s", lastAction)
	}
}

func TestListWorkFiltered(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw, _ := io.ReadAll(r.Body)
		var body map[string]any
		_ = json.Unmarshal(raw, &body)
		if body["action"] != "list" || body["status"] != "queued" {
			t.Fatalf("%v", body)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{"work": []any{}})
	}))
	defer srv.Close()
	c := swissarmynoife.NewClient(srv.URL)
	out, err := c.ListWorkFiltered(map[string]any{"status": "queued"})
	if err != nil {
		t.Fatal(err)
	}
	m := out.(map[string]any)
	if _, ok := m["work"]; !ok {
		t.Fatalf("%v", out)
	}
}
