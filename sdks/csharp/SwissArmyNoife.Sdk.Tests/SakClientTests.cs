using System.Net;
using System.Text;
using System.Text.Json;

namespace SwissArmyNoife.Sdk.Tests;

public class SakClientTests
{
    [Fact]
    public void BaseUrl_StripsTrailingSlash()
    {
        var c = new SakClient("http://127.0.0.1:8787/");
        Assert.Equal("http://127.0.0.1:8787", c.BaseUrl);
    }

    [Fact]
    public async Task Health_CallsEndpoint()
    {
        var handler = new StubHandler((req, _) =>
        {
            Assert.Equal("/health", req.RequestUri!.AbsolutePath);
            return JsonOk("""{"ok":true}""");
        });
        var c = new SakClient("http://example.test", new HttpClient(handler));
        var out_ = await c.HealthAsync();
        Assert.True(out_.GetProperty("ok").GetBoolean());
    }

    [Theory]
    [InlineData("/v1/sak/modules", "modules")]
    [InlineData("/v1/sak/compute/work", "work")]
    [InlineData("/v1/sak/compute/nodes", "nodes")]
    [InlineData("/v1/sak/capacity", "capacity")]
    public async Task ListHelpers_CallPaths(string path, string kind)
    {
        var handler = new StubHandler((req, _) =>
        {
            Assert.Equal(path, req.RequestUri!.AbsolutePath);
            var body = kind switch
            {
                "modules" => """{"modules":[]}""",
                "work" => """{"work":[]}""",
                "nodes" => """{"nodes":[]}""",
                _ => """{"snapshot":{"total_ram_mb":1}}""",
            };
            return JsonOk(body);
        });
        var c = new SakClient("http://example.test", new HttpClient(handler));
        JsonElement out_ = kind switch
        {
            "modules" => await c.ListModulesAsync(),
            "work" => await c.ListWorkAsync(),
            "nodes" => await c.ListNodesAsync(),
            _ => await c.CapacityAsync(),
        };
        Assert.Equal(JsonValueKind.Object, out_.ValueKind);
    }

    [Fact]
    public async Task EnqueueWork_PostsAction()
    {
        string? posted = null;
        var handler = new StubHandler(async (req, ct) =>
        {
            Assert.Equal("/v1/sak/compute/work", req.RequestUri!.AbsolutePath);
            posted = await req.Content!.ReadAsStringAsync(ct);
            return JsonOk("""{"action":"enqueue","work":{"status":"queued"}}""");
        });
        var c = new SakClient("http://example.test", new HttpClient(handler));
        var out_ = await c.EnqueueWorkAsync("echo", new Dictionary<string, object?> { ["n"] = 1 });
        Assert.Equal("enqueue", out_.GetProperty("action").GetString());
        Assert.Contains("\"action\":\"enqueue\"", posted);
        Assert.Contains("\"kind\":\"echo\"", posted);
    }

    [Fact]
    public async Task RequeueAndClaim_PostActions()
    {
        var actions = new List<string>();
        var handler = new StubHandler(async (req, ct) =>
        {
            var text = await req.Content!.ReadAsStringAsync(ct);
            using var doc = JsonDocument.Parse(text);
            actions.Add(doc.RootElement.GetProperty("action").GetString()!);
            return JsonOk("""{"action":"ok","work":{"id":"w1"}}""");
        });
        var c = new SakClient("http://example.test", new HttpClient(handler));
        await c.RequeueWorkAsync("w1");
        await c.ClaimWorkAsync("n1");
        await c.CompleteWorkAsync("w1", "n1");
        await c.GetWorkAsync("w1");
        Assert.Equal(new[] { "requeue", "claim", "complete", "get" }, actions);
    }

    private static HttpResponseMessage JsonOk(string json) =>
        new(HttpStatusCode.OK)
        {
            Content = new StringContent(json, Encoding.UTF8, "application/json"),
        };

    private sealed class StubHandler : HttpMessageHandler
    {
        private readonly Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> _fn;

        public StubHandler(Func<HttpRequestMessage, CancellationToken, HttpResponseMessage> fn)
            : this((r, ct) => Task.FromResult(fn(r, ct)))
        {
        }

        public StubHandler(Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> fn)
        {
            _fn = fn;
        }

        protected override Task<HttpResponseMessage> SendAsync(
            HttpRequestMessage request,
            CancellationToken cancellationToken) =>
            _fn(request, cancellationToken);
    }
}
