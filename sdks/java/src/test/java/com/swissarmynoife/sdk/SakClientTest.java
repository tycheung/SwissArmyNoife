package com.swissarmynoife.sdk;

import com.sun.net.httpserver.HttpServer;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.concurrent.atomic.AtomicReference;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

class SakClientTest {

  @Test
  void stripsTrailingSlash() {
    SakClient c = new SakClient("http://127.0.0.1:8787/");
    assertEquals("http://127.0.0.1:8787", c.baseUrl());
  }

  @Test
  void health() throws Exception {
    try (LocalJsonServer srv = LocalJsonServer.start("/health", "{\"ok\":true}")) {
      SakClient c = new SakClient(srv.baseUrl());
      assertEquals(Boolean.TRUE, c.health().get("ok"));
    }
  }

  @Test
  void listHelpers() throws Exception {
    record Case(String path, String body, ThrowingFn fn) {}
    Case[] cases =
        new Case[] {
          new Case("/v1/sak/modules", "{\"modules\":[]}", SakClient::listModules),
          new Case("/v1/sak/compute/work", "{\"work\":[]}", SakClient::listWork),
          new Case("/v1/sak/compute/nodes", "{\"nodes\":[]}", SakClient::listNodes),
          new Case(
              "/v1/sak/capacity",
              "{\"snapshot\":{\"total_ram_mb\":1}}",
              SakClient::capacity),
        };
    for (Case tc : cases) {
      try (LocalJsonServer srv = LocalJsonServer.start(tc.path, tc.body)) {
        SakClient c = new SakClient(srv.baseUrl());
        Map<String, Object> out = tc.fn.apply(c);
        assertTrue(out != null && !out.isEmpty());
      }
    }
  }

  @Test
  void enqueueWork() throws Exception {
    AtomicReference<String> posted = new AtomicReference<>();
    try (LocalJsonServer srv =
        LocalJsonServer.startPost(
            "/v1/sak/compute/work",
            posted,
            "{\"action\":\"enqueue\",\"work\":{\"status\":\"queued\"}}")) {
      SakClient c = new SakClient(srv.baseUrl());
      Map<String, Object> out = c.enqueueWork("echo", Map.of("n", 1));
      assertEquals("enqueue", out.get("action"));
      assertTrue(posted.get().contains("\"action\":\"enqueue\""));
      assertTrue(posted.get().contains("\"kind\":\"echo\""));
    }
  }

  @Test
  void requeueAndClaim() throws Exception {
    AtomicReference<String> posted = new AtomicReference<>();
    try (LocalJsonServer srv =
        LocalJsonServer.startPost(
            "/v1/sak/compute/work",
            posted,
            "{\"action\":\"ok\",\"work\":{\"id\":\"w1\"}}")) {
      SakClient c = new SakClient(srv.baseUrl());
      c.requeueWork("w1");
      assertTrue(posted.get().contains("requeue"));
      c.claimWork("n1");
      assertTrue(posted.get().contains("claim"));
      c.completeWork("w1", "n1", null);
      assertTrue(posted.get().contains("complete"));
      c.getWork("w1");
      assertTrue(posted.get().contains("\"action\":\"get\""));
    }
  }

  @FunctionalInterface
  interface ThrowingFn {
    Map<String, Object> apply(SakClient c) throws Exception;
  }

  /** Minimal JSON stub server. */
  static final class LocalJsonServer implements AutoCloseable {
    private final HttpServer server;
    private final int port;

    private LocalJsonServer(HttpServer server, int port) {
      this.server = server;
      this.port = port;
    }

    String baseUrl() {
      return "http://127.0.0.1:" + port;
    }

    static LocalJsonServer start(String path, String json) throws IOException {
      HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
      server.createContext(
          path,
          exchange -> {
            byte[] bytes = json.getBytes(StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "application/json");
            exchange.sendResponseHeaders(200, bytes.length);
            try (OutputStream os = exchange.getResponseBody()) {
              os.write(bytes);
            }
          });
      server.start();
      return new LocalJsonServer(server, server.getAddress().getPort());
    }

    static LocalJsonServer startPost(String path, AtomicReference<String> bodyOut, String json)
        throws IOException {
      HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
      server.createContext(
          path,
          exchange -> {
            bodyOut.set(new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8));
            byte[] bytes = json.getBytes(StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "application/json");
            exchange.sendResponseHeaders(200, bytes.length);
            try (OutputStream os = exchange.getResponseBody()) {
              os.write(bytes);
            }
          });
      server.start();
      return new LocalJsonServer(server, server.getAddress().getPort());
    }

    @Override
    public void close() {
      server.stop(0);
    }
  }
}
