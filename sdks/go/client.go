// Package swissarmynoife is the Go HTTP admin + MCP client for SwissArmyNoife (sak330).
package swissarmynoife

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

const defaultHTTP = "http://127.0.0.1:8787"

// Client talks to SwissArmyNoife http-admin.
type Client struct {
	BaseURL    string
	HTTPClient *http.Client
}

// NewClient returns an HTTP admin client. baseURL may be empty for the default.
func NewClient(baseURL string) *Client {
	u := strings.TrimRight(baseURL, "/")
	if u == "" {
		u = defaultHTTP
	}
	return &Client{
		BaseURL:    u,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
	}
}

func (c *Client) getJSON(path string) (any, error) {
	res, err := c.HTTPClient.Get(c.BaseURL + path)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	if res.StatusCode < 200 || res.StatusCode >= 300 {
		return nil, fmt.Errorf("%d: %s", res.StatusCode, string(body))
	}
	var out any
	if err := json.Unmarshal(body, &out); err != nil {
		return nil, err
	}
	return out, nil
}

func (c *Client) postJSON(path string, payload any) (any, error) {
	raw, err := json.Marshal(payload)
	if err != nil {
		return nil, err
	}
	res, err := c.HTTPClient.Post(c.BaseURL+path, "application/json", bytes.NewReader(raw))
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	if res.StatusCode < 200 || res.StatusCode >= 300 {
		return nil, fmt.Errorf("%d: %s", res.StatusCode, string(body))
	}
	var out any
	if err := json.Unmarshal(body, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// Health GET /health
func (c *Client) Health() (any, error) { return c.getJSON("/health") }

// ListModules GET /v1/sak/modules
func (c *Client) ListModules() (any, error) { return c.getJSON("/v1/sak/modules") }

// GetModule GET /v1/sak/modules/{id}
func (c *Client) GetModule(id string) (any, error) {
	return c.getJSON("/v1/sak/modules/" + url.PathEscape(id))
}

// Capacity GET /v1/sak/capacity
func (c *Client) Capacity() (any, error) { return c.getJSON("/v1/sak/capacity") }

// ListWork GET /v1/sak/compute/work
func (c *Client) ListWork() (any, error) { return c.getJSON("/v1/sak/compute/work") }

// ListNodes GET /v1/sak/compute/nodes
func (c *Client) ListNodes() (any, error) { return c.getJSON("/v1/sak/compute/nodes") }

// ComputeWork POST /v1/sak/compute/work
func (c *Client) ComputeWork(body map[string]any) (any, error) {
	return c.postJSON("/v1/sak/compute/work", body)
}

// ComputeNodes POST /v1/sak/compute/nodes
func (c *Client) ComputeNodes(body map[string]any) (any, error) {
	return c.postJSON("/v1/sak/compute/nodes", body)
}

// EnqueueWork POSTs compute_work action=enqueue.
func (c *Client) EnqueueWork(kind string, payload map[string]any) (any, error) {
	if payload == nil {
		payload = map[string]any{}
	}
	return c.ComputeWork(map[string]any{
		"action":  "enqueue",
		"kind":    kind,
		"payload": payload,
	})
}

// ClaimWork POSTs compute_work action=claim.
func (c *Client) ClaimWork(nodeID string) (any, error) {
	return c.ComputeWork(map[string]any{"action": "claim", "node_id": nodeID})
}

// CompleteWork POSTs compute_work action=complete.
func (c *Client) CompleteWork(workID, nodeID string, result map[string]any) (any, error) {
	if result == nil {
		result = map[string]any{}
	}
	return c.ComputeWork(map[string]any{
		"action":  "complete",
		"work_id": workID,
		"node_id": nodeID,
		"result":  result,
	})
}

// GetWork POSTs compute_work action=get.
func (c *Client) GetWork(workID string) (any, error) {
	return c.ComputeWork(map[string]any{"action": "get", "work_id": workID})
}

// RequeueWork POSTs compute_work action=requeue.
func (c *Client) RequeueWork(workID string) (any, error) {
	return c.ComputeWork(map[string]any{"action": "requeue", "work_id": workID})
}

// ListWorkFiltered POSTs compute_work action=list with optional filters.
func (c *Client) ListWorkFiltered(filters map[string]any) (any, error) {
	body := map[string]any{"action": "list"}
	for k, v := range filters {
		body[k] = v
	}
	return c.ComputeWork(body)
}

// ListNodesFiltered POSTs compute_nodes action=list with optional filters.
func (c *Client) ListNodesFiltered(filters map[string]any) (any, error) {
	body := map[string]any{"action": "list"}
	for k, v := range filters {
		body[k] = v
	}
	return c.ComputeNodes(body)
}

// RegisterNode POSTs compute_nodes action=register.
func (c *Client) RegisterNode(label string, caps []string, nodeID, sessionID string) (any, error) {
	body := map[string]any{"action": "register", "label": label}
	if caps != nil {
		body["caps"] = caps
	}
	if nodeID != "" {
		body["node_id"] = nodeID
	}
	if sessionID != "" {
		body["session_id"] = sessionID
	}
	return c.ComputeNodes(body)
}

// HeartbeatNode POSTs compute_nodes action=heartbeat.
func (c *Client) HeartbeatNode(nodeID string) (any, error) {
	return c.ComputeNodes(map[string]any{"action": "heartbeat", "node_id": nodeID})
}
