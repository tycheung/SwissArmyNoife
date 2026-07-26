using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;

namespace SwissArmyNoife.Sdk;

/// <summary>Streamable HTTP MCP client (sak332-c).</summary>
public sealed class SakMcpClient
{
    private const string DefaultMcp = "http://127.0.0.1:8080/mcp";
    private const string Protocol = "2024-11-05";
    private const string SessionHeader = "mcp-session-id";
    private const string Accept = "application/json, text/event-stream";

    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    private readonly HttpClient _http;
    private int _rpcId;
    private string? _sessionId;
    private bool _initialized;

    public string BaseUrl { get; }
    public string? Token { get; set; }
    public bool AutoInitialize { get; set; } = true;
    public string? SessionId => _sessionId;

    public SakMcpClient(string? baseUrl = null, HttpClient? http = null)
    {
        var u = string.IsNullOrWhiteSpace(baseUrl) ? DefaultMcp : baseUrl.TrimEnd('/');
        BaseUrl = u;
        _http = http ?? new HttpClient { Timeout = TimeSpan.FromSeconds(30) };
    }

    public async Task<JsonElement> InitializeAsync(CancellationToken ct = default)
    {
        var result = await RpcAsync(
                "initialize",
                new Dictionary<string, object?>
                {
                    ["protocolVersion"] = Protocol,
                    ["capabilities"] = new Dictionary<string, object?>(),
                    ["clientInfo"] = new Dictionary<string, object?>
                    {
                        ["name"] = "swissarmynoife-csharp",
                        ["version"] = "0.1.0",
                    },
                },
                ct)
            .ConfigureAwait(false);
        await PostAsync(
                new Dictionary<string, object?>
                {
                    ["jsonrpc"] = "2.0",
                    ["method"] = "notifications/initialized",
                },
                notification: true,
                ct)
            .ConfigureAwait(false);
        _initialized = true;
        return result;
    }

    public async Task<string> PingAsync(CancellationToken ct = default)
    {
        var result = await ToolsCallAsync("ping", new Dictionary<string, object?>(), ct)
            .ConfigureAwait(false);
        return ExtractPingText(result);
    }

    public async Task<JsonElement> ToolsListAsync(CancellationToken ct = default)
    {
        await EnsureSessionAsync(ct).ConfigureAwait(false);
        return await RpcAsync("tools/list", new Dictionary<string, object?>(), ct)
            .ConfigureAwait(false);
    }

    public Task<JsonElement> CatalogListAsync(CancellationToken ct = default) =>
        ToolsCallAsync("catalog_list", new Dictionary<string, object?>(), ct);

    private async Task EnsureSessionAsync(CancellationToken ct)
    {
        if (!AutoInitialize || _initialized)
        {
            return;
        }
        await InitializeAsync(ct).ConfigureAwait(false);
    }

    private async Task<JsonElement> ToolsCallAsync(
        string name,
        Dictionary<string, object?> arguments,
        CancellationToken ct)
    {
        await EnsureSessionAsync(ct).ConfigureAwait(false);
        return await RpcAsync(
                "tools/call",
                new Dictionary<string, object?> { ["name"] = name, ["arguments"] = arguments },
                ct)
            .ConfigureAwait(false);
    }

    private async Task<JsonElement> RpcAsync(
        string method,
        Dictionary<string, object?> parameters,
        CancellationToken ct)
    {
        _rpcId++;
        var payload = new Dictionary<string, object?>
        {
            ["jsonrpc"] = "2.0",
            ["id"] = _rpcId,
            ["method"] = method,
            ["params"] = parameters,
        };
        using var res = await PostAsync(payload, notification: false, ct).ConfigureAwait(false);
        var text = await res.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        using var doc = JsonDocument.Parse(text);
        var root = doc.RootElement.Clone();
        CaptureSession(res, root);
        if (root.TryGetProperty("error", out var err))
        {
            var msg = err.ValueKind == JsonValueKind.Object && err.TryGetProperty("message", out var m)
                ? m.GetString()
                : err.ToString();
            throw new InvalidOperationException($"MCP {method} failed: {msg}");
        }
        if (root.TryGetProperty("result", out var result))
        {
            return result.Clone();
        }
        return root;
    }

    private async Task<HttpResponseMessage> PostAsync(
        Dictionary<string, object?> payload,
        bool notification,
        CancellationToken ct)
    {
        using var req = new HttpRequestMessage(HttpMethod.Post, BaseUrl)
        {
            Content = JsonContent.Create(payload, options: JsonOpts),
        };
        req.Headers.Accept.Clear();
        req.Headers.TryAddWithoutValidation("Accept", Accept);
        if (!string.IsNullOrWhiteSpace(Token))
        {
            req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", Token);
        }
        if (!string.IsNullOrWhiteSpace(_sessionId))
        {
            req.Headers.TryAddWithoutValidation(SessionHeader, _sessionId);
        }
        var res = await _http.SendAsync(req, ct).ConfigureAwait(false);
        if (notification && ((int)res.StatusCode is 200 or 202))
        {
            return res;
        }
        if (!res.IsSuccessStatusCode)
        {
            var body = await res.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            res.Dispose();
            throw new HttpRequestException($"{(int)res.StatusCode}: {body}");
        }
        return res;
    }

    private void CaptureSession(HttpResponseMessage res, JsonElement body)
    {
        if (string.IsNullOrWhiteSpace(_sessionId)
            && res.Headers.TryGetValues(SessionHeader, out var vals))
        {
            var sid = vals.FirstOrDefault()?.Trim();
            if (!string.IsNullOrWhiteSpace(sid))
            {
                _sessionId = sid;
            }
        }
        if (string.IsNullOrWhiteSpace(_sessionId))
        {
            var fromBody = SessionIdFromBody(body);
            if (!string.IsNullOrWhiteSpace(fromBody))
            {
                _sessionId = fromBody;
            }
        }
    }

    private static string? SessionIdFromBody(JsonElement body)
    {
        foreach (var key in new[] { "sessionId", "session_id", "mcp-session-id" })
        {
            if (body.TryGetProperty(key, out var v) && v.ValueKind == JsonValueKind.String)
            {
                var s = v.GetString()?.Trim();
                if (!string.IsNullOrWhiteSpace(s))
                {
                    return s;
                }
            }
        }
        if (body.TryGetProperty("result", out var result) && result.ValueKind == JsonValueKind.Object)
        {
            foreach (var key in new[] { "sessionId", "session_id", "mcp-session-id" })
            {
                if (result.TryGetProperty(key, out var v) && v.ValueKind == JsonValueKind.String)
                {
                    var s = v.GetString()?.Trim();
                    if (!string.IsNullOrWhiteSpace(s))
                    {
                        return s;
                    }
                }
            }
        }
        return null;
    }

    private static string ExtractPingText(JsonElement result)
    {
        if (result.ValueKind == JsonValueKind.String)
        {
            return result.GetString() ?? "";
        }
        if (result.ValueKind == JsonValueKind.Object
            && result.TryGetProperty("content", out var content)
            && content.ValueKind == JsonValueKind.Array)
        {
            foreach (var item in content.EnumerateArray())
            {
                if (item.ValueKind == JsonValueKind.Object
                    && item.TryGetProperty("text", out var text)
                    && text.ValueKind == JsonValueKind.String)
                {
                    return text.GetString() ?? "";
                }
            }
        }
        return result.ToString();
    }
}
