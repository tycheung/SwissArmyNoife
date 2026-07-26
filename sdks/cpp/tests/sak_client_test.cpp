#include <catch2/catch_test_macros.hpp>
#include <httplib.h>
#include <thread>

#include "swissarmynoife/sak_client.hpp"

TEST_CASE("SakClient strips slash", "[http]") {
  swissarmynoife::SakClient c("http://127.0.0.1:8787/");
  REQUIRE(c.base_url() == "http://127.0.0.1:8787");
}

TEST_CASE("SakClient health and lists", "[http]") {
  httplib::Server svr;
  svr.Get("/health", [](const httplib::Request&, httplib::Response& res) {
    res.set_content(R"({"ok":true})", "application/json");
  });
  svr.Get("/v1/sak/modules", [](const httplib::Request&, httplib::Response& res) {
    res.set_content(R"({"modules":[]})", "application/json");
  });
  svr.Get("/v1/sak/capacity", [](const httplib::Request&, httplib::Response& res) {
    res.set_content(R"({"snapshot":{"total_ram_mb":1}})", "application/json");
  });
  svr.Get("/v1/sak/compute/work", [](const httplib::Request&, httplib::Response& res) {
    res.set_content(R"({"work":[]})", "application/json");
  });
  svr.Get("/v1/sak/compute/nodes", [](const httplib::Request&, httplib::Response& res) {
    res.set_content(R"({"nodes":[]})", "application/json");
  });
  auto port = svr.bind_to_any_port("127.0.0.1");
  std::thread t([&] { svr.listen_after_bind(); });
  swissarmynoife::SakClient c("http://127.0.0.1:" + std::to_string(port));
  REQUIRE(c.health()["ok"] == true);
  REQUIRE(c.list_modules().contains("modules"));
  REQUIRE(c.capacity().contains("snapshot"));
  REQUIRE(c.list_work().contains("work"));
  REQUIRE(c.list_nodes().contains("nodes"));
  svr.stop();
  t.join();
}

TEST_CASE("SakClient enqueue and claim", "[http]") {
  std::string last;
  httplib::Server svr;
  svr.Post("/v1/sak/compute/work", [&](const httplib::Request& req, httplib::Response& res) {
    last = req.body;
    res.set_content(R"({"action":"ok","work":{"id":"w1"}})", "application/json");
  });
  auto port = svr.bind_to_any_port("127.0.0.1");
  std::thread t([&] { svr.listen_after_bind(); });
  swissarmynoife::SakClient c("http://127.0.0.1:" + std::to_string(port));
  auto out = c.enqueue_work("echo", {{"n", 1}});
  REQUIRE(out["action"] == "ok");
  REQUIRE(last.find("\"action\":\"enqueue\"") != std::string::npos);
  c.requeue_work("w1");
  REQUIRE(last.find("requeue") != std::string::npos);
  c.claim_work("n1");
  REQUIRE(last.find("claim") != std::string::npos);
  c.complete_work("w1", "n1");
  REQUIRE(last.find("complete") != std::string::npos);
  c.get_work("w1");
  REQUIRE(last.find("\"action\":\"get\"") != std::string::npos);
  svr.stop();
  t.join();
}
