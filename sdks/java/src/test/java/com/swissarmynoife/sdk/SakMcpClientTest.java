package com.swissarmynoife.sdk;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;
import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.io.OutputStream;
import java.lang.reflect.Type;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

class SakMcpClientTest {
  private static final Gson GSON = new Gson();
  private static final Type MAP_TYPE = new TypeToken<Map<String, Object>>() {}.getType();

  @Test
  void pingWithSession() throws Exception {
    HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        "/",
        exchange -> {
          String raw = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
          Map<String, Object> body = GSON.fromJson(raw, MAP_TYPE);
          String method = String.valueOf(body.get("method"));
          if ("initialize".equals(method)) {
            writeJson(exchange, 200, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}", "sess-java-1");
          } else if ("notifications/initialized".equals(method)) {
            exchange.sendResponseHeaders(202, -1);
            exchange.close();
          } else if ("tools/call".equals(method)) {
            assertEquals("sess-java-1", exchange.getRequestHeaders().getFirst("mcp-session-id"));
            writeJson(
                exchange,
                200,
                "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"pong\"}]}}",
                null);
          } else {
            exchange.sendResponseHeaders(500, -1);
            exchange.close();
          }
        });
    server.start();
    try {
      SakMcpClient mcp = new SakMcpClient("http://127.0.0.1:" + server.getAddress().getPort() + "/");
      assertEquals("pong", mcp.ping());
      assertEquals("sess-java-1", mcp.getSessionId());
    } finally {
      server.stop(0);
    }
  }

  @Test
  void toolsListNoAutoInit() throws Exception {
    HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        "/",
        exchange -> {
          String raw = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
          Map<String, Object> body = GSON.fromJson(raw, MAP_TYPE);
          assertEquals("tools/list", body.get("method"));
          writeJson(exchange, 200, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}", null);
        });
    server.start();
    try {
      SakMcpClient mcp = new SakMcpClient("http://127.0.0.1:" + server.getAddress().getPort());
      mcp.setAutoInitialize(false);
      @SuppressWarnings("unchecked")
      Map<String, Object> out = (Map<String, Object>) mcp.toolsList();
      assertTrue(out.containsKey("tools"));
    } finally {
      server.stop(0);
    }
  }

  @Test
  void catalogList() throws Exception {
    HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.createContext(
        "/",
        exchange -> {
          String raw = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
          Map<String, Object> body = GSON.fromJson(raw, MAP_TYPE);
          String method = String.valueOf(body.get("method"));
          if ("initialize".equals(method)) {
            writeJson(exchange, 200, "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}", "s2");
          } else if ("notifications/initialized".equals(method)) {
            exchange.sendResponseHeaders(202, -1);
            exchange.close();
          } else {
            writeJson(
                exchange,
                200,
                "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"offers\":[]}}",
                null);
          }
        });
    server.start();
    try {
      SakMcpClient mcp = new SakMcpClient("http://127.0.0.1:" + server.getAddress().getPort());
      @SuppressWarnings("unchecked")
      Map<String, Object> out = (Map<String, Object>) mcp.catalogList();
      assertTrue(out.containsKey("offers"));
    } finally {
      server.stop(0);
    }
  }

  private static void writeJson(
      com.sun.net.httpserver.HttpExchange exchange, int code, String json, String session)
      throws IOException {
    if (session != null) {
      exchange.getResponseHeaders().add("mcp-session-id", session);
    }
    exchange.getResponseHeaders().add("Content-Type", "application/json");
    byte[] bytes = json.getBytes(StandardCharsets.UTF_8);
    exchange.sendResponseHeaders(code, bytes.length);
    try (OutputStream os = exchange.getResponseBody()) {
      os.write(bytes);
    }
  }
}
