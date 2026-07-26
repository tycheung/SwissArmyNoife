using System.Net.Http.Json;
using System.Text.Json;

namespace SwissArmyNoife.Sdk;

/// <summary>HTTP admin client for SwissArmyNoife (sak332-b).</summary>
public sealed class SakClient
{
    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    private readonly HttpClient _http;

    public string BaseUrl { get; }

    public SakClient(string? baseUrl = null, HttpClient? http = null)
    {
        var u = string.IsNullOrWhiteSpace(baseUrl) ? "http://127.0.0.1:8787" : baseUrl.TrimEnd('/');
        BaseUrl = u;
        _http = http ?? new HttpClient { Timeout = TimeSpan.FromSeconds(30) };
    }

    public Task<JsonElement> HealthAsync(CancellationToken ct = default) =>
        GetJsonAsync("/health", ct);

    public Task<JsonElement> ListModulesAsync(CancellationToken ct = default) =>
        GetJsonAsync("/v1/sak/modules", ct);

    public Task<JsonElement> GetModuleAsync(string id, CancellationToken ct = default) =>
        GetJsonAsync($"/v1/sak/modules/{Uri.EscapeDataString(id)}", ct);

    public Task<JsonElement> CapacityAsync(CancellationToken ct = default) =>
        GetJsonAsync("/v1/sak/capacity", ct);

    public Task<JsonElement> ListWorkAsync(CancellationToken ct = default) =>
        GetJsonAsync("/v1/sak/compute/work", ct);

    public Task<JsonElement> ListNodesAsync(CancellationToken ct = default) =>
        GetJsonAsync("/v1/sak/compute/nodes", ct);

    public Task<JsonElement> ComputeWorkAsync(
        Dictionary<string, object?> body,
        CancellationToken ct = default) =>
        PostJsonAsync("/v1/sak/compute/work", body, ct);

    public Task<JsonElement> ComputeNodesAsync(
        Dictionary<string, object?> body,
        CancellationToken ct = default) =>
        PostJsonAsync("/v1/sak/compute/nodes", body, ct);

    public Task<JsonElement> EnqueueWorkAsync(
        string kind,
        Dictionary<string, object?>? payload = null,
        CancellationToken ct = default) =>
        ComputeWorkAsync(
            new Dictionary<string, object?>
            {
                ["action"] = "enqueue",
                ["kind"] = kind,
                ["payload"] = payload ?? new Dictionary<string, object?>(),
            },
            ct);

    public Task<JsonElement> ClaimWorkAsync(string nodeId, CancellationToken ct = default) =>
        ComputeWorkAsync(
            new Dictionary<string, object?> { ["action"] = "claim", ["node_id"] = nodeId },
            ct);

    public Task<JsonElement> CompleteWorkAsync(
        string workId,
        string nodeId,
        Dictionary<string, object?>? result = null,
        CancellationToken ct = default) =>
        ComputeWorkAsync(
            new Dictionary<string, object?>
            {
                ["action"] = "complete",
                ["work_id"] = workId,
                ["node_id"] = nodeId,
                ["result"] = result ?? new Dictionary<string, object?>(),
            },
            ct);

    public Task<JsonElement> GetWorkAsync(string workId, CancellationToken ct = default) =>
        ComputeWorkAsync(
            new Dictionary<string, object?> { ["action"] = "get", ["work_id"] = workId },
            ct);

    public Task<JsonElement> RequeueWorkAsync(string workId, CancellationToken ct = default) =>
        ComputeWorkAsync(
            new Dictionary<string, object?> { ["action"] = "requeue", ["work_id"] = workId },
            ct);

    public Task<JsonElement> ListWorkFilteredAsync(
        Dictionary<string, object?>? filters = null,
        CancellationToken ct = default)
    {
        var body = new Dictionary<string, object?> { ["action"] = "list" };
        if (filters is not null)
        {
            foreach (var kv in filters)
            {
                body[kv.Key] = kv.Value;
            }
        }
        return ComputeWorkAsync(body, ct);
    }

    public Task<JsonElement> ListNodesFilteredAsync(
        Dictionary<string, object?>? filters = null,
        CancellationToken ct = default)
    {
        var body = new Dictionary<string, object?> { ["action"] = "list" };
        if (filters is not null)
        {
            foreach (var kv in filters)
            {
                body[kv.Key] = kv.Value;
            }
        }
        return ComputeNodesAsync(body, ct);
    }

    public Task<JsonElement> RegisterNodeAsync(
        string label,
        IEnumerable<string>? caps = null,
        string? nodeId = null,
        string? sessionId = null,
        CancellationToken ct = default)
    {
        var body = new Dictionary<string, object?> { ["action"] = "register", ["label"] = label };
        if (caps is not null)
        {
            body["caps"] = caps.ToList();
        }
        if (!string.IsNullOrWhiteSpace(nodeId))
        {
            body["node_id"] = nodeId;
        }
        if (!string.IsNullOrWhiteSpace(sessionId))
        {
            body["session_id"] = sessionId;
        }
        return ComputeNodesAsync(body, ct);
    }

    public Task<JsonElement> HeartbeatNodeAsync(string nodeId, CancellationToken ct = default) =>
        ComputeNodesAsync(
            new Dictionary<string, object?> { ["action"] = "heartbeat", ["node_id"] = nodeId },
            ct);

    private async Task<JsonElement> GetJsonAsync(string path, CancellationToken ct)
    {
        using var res = await _http.GetAsync(BaseUrl + path, ct).ConfigureAwait(false);
        var text = await res.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        if (!res.IsSuccessStatusCode)
        {
            throw new HttpRequestException($"{(int)res.StatusCode}: {text}");
        }
        return JsonSerializer.Deserialize<JsonElement>(text, JsonOpts);
    }

    private async Task<JsonElement> PostJsonAsync(
        string path,
        Dictionary<string, object?> payload,
        CancellationToken ct)
    {
        using var res = await _http
            .PostAsJsonAsync(BaseUrl + path, payload, JsonOpts, ct)
            .ConfigureAwait(false);
        var text = await res.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        if (!res.IsSuccessStatusCode)
        {
            throw new HttpRequestException($"{(int)res.StatusCode}: {text}");
        }
        return JsonSerializer.Deserialize<JsonElement>(text, JsonOpts);
    }
}
