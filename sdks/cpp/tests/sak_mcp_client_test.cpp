#include <catch2/catch_test_macros.hpp>
#include <httplib.h>
#include <nlohmann/json.hpp>
#include <thread>

#include "swissarmynoife/sak_mcp_client.hpp"

TEST_CASE("SakMcpClient ping negotiates session", "[mcp]") {
  httplib::Server svr;
  svr.Post("/mcp", [](const httplib::Request& req, httplib::Response& res) {
    auto body = nlohmann::json::parse(req.body);
    auto method = body.value("method", "");
    if (method == "initialize") {
      res.set_header("mcp-session-id", "sess-cpp-1");
      res.set_content(R"({"jsonrpc":"2.0","id":1,"result":{}})", "application/json");
    } else if (method == "notifications/initialized") {
      res.status = 202;
    } else if (method == "tools/call") {
      REQUIRE(req.get_header_value("mcp-session-id") == "sess-cpp-1");
      res.set_content(
          R"({"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}]}})",
          "application/json");
    } else {
      res.status = 500;
    }
  });
  auto port = svr.bind_to_any_port("127.0.0.1");
  std::thread t([&] { svr.listen_after_bind(); });
  swissarmynoife::SakMcpClient mcp("http://127.0.0.1:" + std::to_string(port) + "/mcp");
  REQUIRE(mcp.ping() == "pong");
  REQUIRE(mcp.session_id().has_value());
  REQUIRE(*mcp.session_id() == "sess-cpp-1");
  svr.stop();
  t.join();
}

TEST_CASE("SakMcpClient tools_list no auto init", "[mcp]") {
  httplib::Server svr;
  svr.Post("/mcp", [](const httplib::Request& req, httplib::Response& res) {
    auto body = nlohmann::json::parse(req.body);
    REQUIRE(body.value("method", "") == "tools/list");
    res.set_content(R"({"jsonrpc":"2.0","id":1,"result":{"tools":[]}})", "application/json");
  });
  auto port = svr.bind_to_any_port("127.0.0.1");
  std::thread t([&] { svr.listen_after_bind(); });
  swissarmynoife::SakMcpClient mcp("http://127.0.0.1:" + std::to_string(port) + "/mcp");
  mcp.set_auto_initialize(false);
  auto out = mcp.tools_list();
  REQUIRE(out.contains("tools"));
  svr.stop();
  t.join();
}

TEST_CASE("SakMcpClient catalog_list", "[mcp]") {
  httplib::Server svr;
  svr.Post("/mcp", [](const httplib::Request& req, httplib::Response& res) {
    auto body = nlohmann::json::parse(req.body);
    auto method = body.value("method", "");
    if (method == "initialize") {
      res.set_header("mcp-session-id", "s2");
      res.set_content(R"({"jsonrpc":"2.0","id":1,"result":{}})", "application/json");
    } else if (method == "notifications/initialized") {
      res.status = 202;
    } else {
      res.set_content(R"({"jsonrpc":"2.0","id":2,"result":{"offers":[]}})", "application/json");
    }
  });
  auto port = svr.bind_to_any_port("127.0.0.1");
  std::thread t([&] { svr.listen_after_bind(); });
  swissarmynoife::SakMcpClient mcp("http://127.0.0.1:" + std::to_string(port) + "/mcp");
  auto out = mcp.catalog_list();
  REQUIRE(out.contains("offers"));
  svr.stop();
  t.join();
}
