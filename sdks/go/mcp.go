package swissarmynoife

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

const (
	defaultMCPURL         = "http://127.0.0.1:8080/mcp"
	mcpProtocolVersion    = "2024-11-05"
	mcpSessionHeader      = "mcp-session-id"
	mcpAccept             = "application/json, text/event-stream"
)

// McpClient is a Streamable HTTP MCP client (sak330-c).
type McpClient struct {
	BaseURL         string
	Token           string
	HTTPClient      *http.Client
	AutoInitialize  bool
	rpcID           int
	sessionID       string
	initialized     bool
}

// NewMcpClient returns an MCP client. baseURL may be empty for the default.
// AutoInitialize defaults to true.
func NewMcpClient(baseURL string) *McpClient {
	u := strings.TrimRight(baseURL, "/")
	if u == "" {
		u = defaultMCPURL
	}
	return &McpClient{
		BaseURL:        u,
		HTTPClient:     &http.Client{Timeout: 30 * time.Second},
		AutoInitialize: true,
	}
}

// SessionID returns the negotiated mcp-session-id, if any.
func (m *McpClient) SessionID() string { return m.sessionID }

func (m *McpClient) authHeaders() http.Header {
	h := make(http.Header)
	h.Set("Content-Type", "application/json")
	h.Set("Accept", mcpAccept)
	if m.Token != "" {
		h.Set("Authorization", "Bearer "+m.Token)
	}
	if m.sessionID != "" {
		h.Set(mcpSessionHeader, m.sessionID)
	}
	return h
}

func (m *McpClient) post(payload map[string]any, notification bool) (*http.Response, error) {
	raw, err := json.Marshal(payload)
	if err != nil {
		return nil, err
	}
	req, err := http.NewRequest(http.MethodPost, m.BaseURL, bytes.NewReader(raw))
	if err != nil {
		return nil, err
	}
	req.Header = m.authHeaders()
	res, err := m.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	if notification && (res.StatusCode == 200 || res.StatusCode == 202) {
		return res, nil
	}
	if res.StatusCode < 200 || res.StatusCode >= 300 {
		body, _ := io.ReadAll(res.Body)
		res.Body.Close()
		return nil, fmt.Errorf("%d: %s", res.StatusCode, string(body))
	}
	return res, nil
}

func (m *McpClient) captureSession(res *http.Response, body any) {
	if m.sessionID == "" {
		if sid := strings.TrimSpace(res.Header.Get(mcpSessionHeader)); sid != "" {
			m.sessionID = sid
		}
	}
	if m.sessionID == "" {
		if sid := sessionIDFromBody(body); sid != "" {
			m.sessionID = sid
		}
	}
}

func sessionIDFromBody(body any) string {
	obj, ok := body.(map[string]any)
	if !ok {
		return ""
	}
	for _, key := range []string{"sessionId", "session_id", "mcp-session-id"} {
		if v, ok := obj[key].(string); ok && strings.TrimSpace(v) != "" {
			return strings.TrimSpace(v)
		}
	}
	if result, ok := obj["result"].(map[string]any); ok {
		for _, key := range []string{"sessionId", "session_id", "mcp-session-id"} {
			if v, ok := result[key].(string); ok && strings.TrimSpace(v) != "" {
				return strings.TrimSpace(v)
			}
		}
	}
	return ""
}

func (m *McpClient) rpc(method string, params map[string]any) (any, error) {
	if params == nil {
		params = map[string]any{}
	}
	m.rpcID++
	payload := map[string]any{
		"jsonrpc": "2.0",
		"id":      m.rpcID,
		"method":  method,
		"params":  params,
	}
	res, err := m.post(payload, false)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()
	raw, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	var body any
	if err := json.Unmarshal(raw, &body); err != nil {
		return nil, err
	}
	m.captureSession(res, body)
	obj, ok := body.(map[string]any)
	if !ok {
		return body, nil
	}
	if errObj, has := obj["error"]; has {
		msg := fmt.Sprint(errObj)
		if em, ok := errObj.(map[string]any); ok {
			if m0, ok := em["message"].(string); ok {
				msg = m0
			}
		}
		return nil, fmt.Errorf("MCP %s failed: %s", method, msg)
	}
	if result, has := obj["result"]; has {
		return result, nil
	}
	return body, nil
}

// Initialize negotiates a Streamable HTTP session.
func (m *McpClient) Initialize() (any, error) {
	result, err := m.rpc("initialize", map[string]any{
		"protocolVersion": mcpProtocolVersion,
		"capabilities":    map[string]any{},
		"clientInfo": map[string]any{
			"name":    "swissarmynoife-go",
			"version": "0.1.0",
		},
	})
	if err != nil {
		return nil, err
	}
	res, err := m.post(map[string]any{
		"jsonrpc": "2.0",
		"method":  "notifications/initialized",
	}, true)
	if err != nil {
		return nil, err
	}
	res.Body.Close()
	m.initialized = true
	return result, nil
}

func (m *McpClient) ensureSession() error {
	if !m.AutoInitialize || m.initialized {
		return nil
	}
	_, err := m.Initialize()
	return err
}

func (m *McpClient) toolsCall(name string, arguments map[string]any) (any, error) {
	if err := m.ensureSession(); err != nil {
		return nil, err
	}
	if arguments == nil {
		arguments = map[string]any{}
	}
	return m.rpc("tools/call", map[string]any{
		"name":      name,
		"arguments": arguments,
	})
}

// Ping calls tools/call ping and returns extracted text.
func (m *McpClient) Ping() (string, error) {
	result, err := m.toolsCall("ping", nil)
	if err != nil {
		return "", err
	}
	return extractPingText(result), nil
}

// ToolsList calls MCP tools/list.
func (m *McpClient) ToolsList() (any, error) {
	if err := m.ensureSession(); err != nil {
		return nil, err
	}
	return m.rpc("tools/list", nil)
}

// CatalogList calls tools/call catalog_list.
func (m *McpClient) CatalogList() (any, error) {
	return m.toolsCall("catalog_list", nil)
}

func extractPingText(result any) string {
	if s, ok := result.(string); ok {
		return s
	}
	obj, ok := result.(map[string]any)
	if !ok {
		return fmt.Sprint(result)
	}
	content, ok := obj["content"].([]any)
	if !ok {
		return fmt.Sprint(result)
	}
	for _, item := range content {
		im, ok := item.(map[string]any)
		if !ok {
			continue
		}
		if text, ok := im["text"].(string); ok {
			return text
		}
	}
	return fmt.Sprint(result)
}
