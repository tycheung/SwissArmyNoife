package com.swissarmynoife.sdk;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;

import java.io.IOException;
import java.lang.reflect.Type;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** Streamable HTTP MCP client (sak331-c). */
public final class SakMcpClient {
  private static final String DEFAULT_MCP = "http://127.0.0.1:8080/mcp";
  private static final String PROTOCOL = "2024-11-05";
  private static final String SESSION_HEADER = "mcp-session-id";
  private static final String ACCEPT = "application/json, text/event-stream";
  private static final Type MAP_TYPE = new TypeToken<Map<String, Object>>() {}.getType();
  private static final Gson GSON = new Gson();

  private final String baseUrl;
  private final HttpClient http;
  private String token;
  private boolean autoInitialize = true;
  private int rpcId;
  private String sessionId;
  private boolean initialized;

  public SakMcpClient(String baseUrl) {
    String u = baseUrl == null || baseUrl.isBlank() ? DEFAULT_MCP : baseUrl;
    while (u.endsWith("/")) {
      u = u.substring(0, u.length() - 1);
    }
    this.baseUrl = u;
    this.http = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build();
  }

  public void setToken(String token) {
    this.token = token;
  }

  public void setAutoInitialize(boolean autoInitialize) {
    this.autoInitialize = autoInitialize;
  }

  public String getSessionId() {
    return sessionId;
  }

  public Object initialize() throws IOException, InterruptedException {
    Object result =
        rpc(
            "initialize",
            Map.of(
                "protocolVersion",
                PROTOCOL,
                "capabilities",
                Map.of(),
                "clientInfo",
                Map.of("name", "swissarmynoife-java", "version", "0.1.0")));
    post(
        Map.of("jsonrpc", "2.0", "method", "notifications/initialized"),
        true);
    initialized = true;
    return result;
  }

  public String ping() throws IOException, InterruptedException {
    return extractPingText(toolsCall("ping", Map.of()));
  }

  public Object toolsList() throws IOException, InterruptedException {
    ensureSession();
    return rpc("tools/list", Map.of());
  }

  public Object catalogList() throws IOException, InterruptedException {
    return toolsCall("catalog_list", Map.of());
  }

  private void ensureSession() throws IOException, InterruptedException {
    if (!autoInitialize || initialized) {
      return;
    }
    initialize();
  }

  private Object toolsCall(String name, Map<String, Object> arguments)
      throws IOException, InterruptedException {
    ensureSession();
    return rpc("tools/call", Map.of("name", name, "arguments", arguments));
  }

  private Object rpc(String method, Map<String, Object> params)
      throws IOException, InterruptedException {
    rpcId++;
    Map<String, Object> payload = new HashMap<>();
    payload.put("jsonrpc", "2.0");
    payload.put("id", rpcId);
    payload.put("method", method);
    payload.put("params", params == null ? Map.of() : params);
    HttpResponse<String> res = post(payload, false);
    Map<String, Object> body = GSON.fromJson(res.body(), MAP_TYPE);
    captureSession(res, body);
    if (body != null && body.containsKey("error")) {
      Object err = body.get("error");
      String msg = String.valueOf(err);
      if (err instanceof Map<?, ?> em && em.get("message") != null) {
        msg = String.valueOf(em.get("message"));
      }
      throw new IOException("MCP " + method + " failed: " + msg);
    }
    if (body != null && body.containsKey("result")) {
      return body.get("result");
    }
    return body;
  }

  private HttpResponse<String> post(Map<String, Object> payload, boolean notification)
      throws IOException, InterruptedException {
    HttpRequest.Builder b =
        HttpRequest.newBuilder(URI.create(baseUrl))
            .timeout(Duration.ofSeconds(30))
            .header("Content-Type", "application/json")
            .header("Accept", ACCEPT)
            .POST(HttpRequest.BodyPublishers.ofString(GSON.toJson(payload)));
    if (token != null && !token.isBlank()) {
      b.header("Authorization", "Bearer " + token);
    }
    if (sessionId != null && !sessionId.isBlank()) {
      b.header(SESSION_HEADER, sessionId);
    }
    HttpResponse<String> res = http.send(b.build(), HttpResponse.BodyHandlers.ofString());
    if (notification && (res.statusCode() == 200 || res.statusCode() == 202)) {
      return res;
    }
    if (res.statusCode() < 200 || res.statusCode() >= 300) {
      throw new IOException(res.statusCode() + ": " + res.body());
    }
    return res;
  }

  private void captureSession(HttpResponse<String> res, Map<String, Object> body) {
    if (sessionId == null || sessionId.isBlank()) {
      res.headers()
          .firstValue(SESSION_HEADER)
          .ifPresent(s -> {
            if (s != null && !s.isBlank()) {
              sessionId = s.trim();
            }
          });
    }
    if ((sessionId == null || sessionId.isBlank()) && body != null) {
      String fromBody = sessionIdFromBody(body);
      if (fromBody != null) {
        sessionId = fromBody;
      }
    }
  }

  @SuppressWarnings("unchecked")
  private static String sessionIdFromBody(Map<String, Object> body) {
    for (String key : List.of("sessionId", "session_id", "mcp-session-id")) {
      Object v = body.get(key);
      if (v instanceof String s && !s.isBlank()) {
        return s.trim();
      }
    }
    Object result = body.get("result");
    if (result instanceof Map<?, ?> rm) {
      Map<String, Object> resultMap = (Map<String, Object>) rm;
      for (String key : List.of("sessionId", "session_id", "mcp-session-id")) {
        Object v = resultMap.get(key);
        if (v instanceof String s && !s.isBlank()) {
          return s.trim();
        }
      }
    }
    return null;
  }

  @SuppressWarnings("unchecked")
  private static String extractPingText(Object result) {
    if (result instanceof String s) {
      return s;
    }
    if (!(result instanceof Map<?, ?> obj)) {
      return String.valueOf(result);
    }
    Object content = obj.get("content");
    if (content instanceof List<?> list) {
      for (Object item : list) {
        if (item instanceof Map<?, ?> im) {
          Object text = im.get("text");
          if (text instanceof String t) {
            return t;
          }
        }
      }
    }
    return String.valueOf(result);
  }
}
