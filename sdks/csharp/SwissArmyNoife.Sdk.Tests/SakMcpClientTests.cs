using System.Net;
using System.Text;
using System.Text.Json;

namespace SwissArmyNoife.Sdk.Tests;

public class SakMcpClientTests
{
    [Fact]
    public async Task Ping_NegotiatesSession()
    {
        var n = 0;
        var handler = new StubHandler(async (req, ct) =>
        {
            var raw = await req.Content!.ReadAsStringAsync(ct);
            using var doc = JsonDocument.Parse(raw);
            var method = doc.RootElement.GetProperty("method").GetString();
            n++;
            return method switch
            {
                "initialize" => JsonOk(
                    """{"jsonrpc":"2.0","id":1,"result":{}}""",
                    session: "sess-cs-1"),
                "notifications/initialized" => new HttpResponseMessage(HttpStatusCode.Accepted),
                "tools/call" => CheckSessionAndPong(req),
                _ => throw new InvalidOperationException(method),
            };
        });
        var mcp = new SakMcpClient("http://example.test/mcp", new HttpClient(handler));
        Assert.Equal("pong", await mcp.PingAsync());
        Assert.Equal("sess-cs-1", mcp.SessionId);
        Assert.True(n >= 3);
    }

    [Fact]
    public async Task ToolsList_NoAutoInit()
    {
        var handler = new StubHandler(async (req, ct) =>
        {
            var raw = await req.Content!.ReadAsStringAsync(ct);
            using var doc = JsonDocument.Parse(raw);
            Assert.Equal("tools/list", doc.RootElement.GetProperty("method").GetString());
            return JsonOk("""{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}""");
        });
        var mcp = new SakMcpClient("http://example.test/mcp", new HttpClient(handler))
        {
            AutoInitialize = false,
        };
        var out_ = await mcp.ToolsListAsync();
        Assert.True(out_.TryGetProperty("tools", out _));
    }

    [Fact]
    public async Task CatalogList_Works()
    {
        var handler = new StubHandler(async (req, ct) =>
        {
            var raw = await req.Content!.ReadAsStringAsync(ct);
            using var doc = JsonDocument.Parse(raw);
            var method = doc.RootElement.GetProperty("method").GetString();
            return method switch
            {
                "initialize" => JsonOk("""{"jsonrpc":"2.0","id":1,"result":{}}""", "s2"),
                "notifications/initialized" => new HttpResponseMessage(HttpStatusCode.Accepted),
                "tools/call" => JsonOk("""{"jsonrpc":"2.0","id":2,"result":{"offers":[]}}"""),
                _ => throw new InvalidOperationException(method),
            };
        });
        var mcp = new SakMcpClient("http://example.test/mcp", new HttpClient(handler));
        var out_ = await mcp.CatalogListAsync();
        Assert.True(out_.TryGetProperty("offers", out _));
    }

    private static HttpResponseMessage CheckSessionAndPong(HttpRequestMessage req)
    {
        Assert.True(req.Headers.TryGetValues("mcp-session-id", out var vals));
        Assert.Equal("sess-cs-1", vals.First());
        return JsonOk(
            """{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}]}}""");
    }

    private static HttpResponseMessage JsonOk(string json, string? session = null)
    {
        var res = new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(json, Encoding.UTF8, "application/json"),
        };
        if (session is not null)
        {
            res.Headers.TryAddWithoutValidation("mcp-session-id", session);
        }
        return res;
    }

    private sealed class StubHandler : HttpMessageHandler
    {
        private readonly Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> _fn;

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
