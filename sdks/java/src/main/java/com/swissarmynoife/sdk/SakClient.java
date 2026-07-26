package com.swissarmynoife.sdk;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;

import java.io.IOException;
import java.lang.reflect.Type;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** HTTP admin client for SwissArmyNoife (sak331). */
public final class SakClient {
  private static final String DEFAULT_HTTP = "http://127.0.0.1:8787";
  private static final Type MAP_TYPE = new TypeToken<Map<String, Object>>() {}.getType();
  private static final Gson GSON = new Gson();

  private final String baseUrl;
  private final HttpClient http;

  public SakClient(String baseUrl) {
    String u = baseUrl == null || baseUrl.isBlank() ? DEFAULT_HTTP : baseUrl;
    while (u.endsWith("/")) {
      u = u.substring(0, u.length() - 1);
    }
    this.baseUrl = u;
    this.http = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(30)).build();
  }

  public String baseUrl() {
    return baseUrl;
  }

  public Map<String, Object> health() throws IOException, InterruptedException {
    return getJson("/health");
  }

  public Map<String, Object> listModules() throws IOException, InterruptedException {
    return getJson("/v1/sak/modules");
  }

  public Map<String, Object> getModule(String id) throws IOException, InterruptedException {
    return getJson("/v1/sak/modules/" + URLEncoder.encode(id, StandardCharsets.UTF_8));
  }

  public Map<String, Object> capacity() throws IOException, InterruptedException {
    return getJson("/v1/sak/capacity");
  }

  public Map<String, Object> listWork() throws IOException, InterruptedException {
    return getJson("/v1/sak/compute/work");
  }

  public Map<String, Object> listNodes() throws IOException, InterruptedException {
    return getJson("/v1/sak/compute/nodes");
  }

  public Map<String, Object> computeWork(Map<String, Object> body)
      throws IOException, InterruptedException {
    return postJson("/v1/sak/compute/work", body);
  }

  public Map<String, Object> computeNodes(Map<String, Object> body)
      throws IOException, InterruptedException {
    return postJson("/v1/sak/compute/nodes", body);
  }

  public Map<String, Object> enqueueWork(String kind, Map<String, Object> payload)
      throws IOException, InterruptedException {
    Map<String, Object> body = new HashMap<>();
    body.put("action", "enqueue");
    body.put("kind", kind);
    body.put("payload", payload == null ? Map.of() : payload);
    return computeWork(body);
  }

  public Map<String, Object> claimWork(String nodeId) throws IOException, InterruptedException {
    return computeWork(Map.of("action", "claim", "node_id", nodeId));
  }

  public Map<String, Object> completeWork(String workId, String nodeId, Map<String, Object> result)
      throws IOException, InterruptedException {
    Map<String, Object> body = new HashMap<>();
    body.put("action", "complete");
    body.put("work_id", workId);
    body.put("node_id", nodeId);
    body.put("result", result == null ? Map.of() : result);
    return computeWork(body);
  }

  public Map<String, Object> getWork(String workId) throws IOException, InterruptedException {
    return computeWork(Map.of("action", "get", "work_id", workId));
  }

  public Map<String, Object> requeueWork(String workId) throws IOException, InterruptedException {
    return computeWork(Map.of("action", "requeue", "work_id", workId));
  }

  public Map<String, Object> listWorkFiltered(Map<String, Object> filters)
      throws IOException, InterruptedException {
    Map<String, Object> body = new HashMap<>();
    body.put("action", "list");
    if (filters != null) {
      body.putAll(filters);
    }
    return computeWork(body);
  }

  public Map<String, Object> listNodesFiltered(Map<String, Object> filters)
      throws IOException, InterruptedException {
    Map<String, Object> body = new HashMap<>();
    body.put("action", "list");
    if (filters != null) {
      body.putAll(filters);
    }
    return computeNodes(body);
  }

  public Map<String, Object> registerNode(
      String label, List<String> caps, String nodeId, String sessionId)
      throws IOException, InterruptedException {
    Map<String, Object> body = new HashMap<>();
    body.put("action", "register");
    body.put("label", label);
    if (caps != null) {
      body.put("caps", caps);
    }
    if (nodeId != null && !nodeId.isBlank()) {
      body.put("node_id", nodeId);
    }
    if (sessionId != null && !sessionId.isBlank()) {
      body.put("session_id", sessionId);
    }
    return computeNodes(body);
  }

  public Map<String, Object> heartbeatNode(String nodeId)
      throws IOException, InterruptedException {
    return computeNodes(Map.of("action", "heartbeat", "node_id", nodeId));
  }

  private Map<String, Object> getJson(String path) throws IOException, InterruptedException {
    HttpRequest req =
        HttpRequest.newBuilder(URI.create(baseUrl + path))
            .timeout(Duration.ofSeconds(30))
            .GET()
            .build();
    HttpResponse<String> res = http.send(req, HttpResponse.BodyHandlers.ofString());
    if (res.statusCode() < 200 || res.statusCode() >= 300) {
      throw new IOException(res.statusCode() + ": " + res.body());
    }
    return GSON.fromJson(res.body(), MAP_TYPE);
  }

  private Map<String, Object> postJson(String path, Map<String, Object> payload)
      throws IOException, InterruptedException {
    HttpRequest req =
        HttpRequest.newBuilder(URI.create(baseUrl + path))
            .timeout(Duration.ofSeconds(30))
            .header("Content-Type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(GSON.toJson(payload)))
            .build();
    HttpResponse<String> res = http.send(req, HttpResponse.BodyHandlers.ofString());
    if (res.statusCode() < 200 || res.statusCode() >= 300) {
      throw new IOException(res.statusCode() + ": " + res.body());
    }
    return GSON.fromJson(res.body(), MAP_TYPE);
  }
}
