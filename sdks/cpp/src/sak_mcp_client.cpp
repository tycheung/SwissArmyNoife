#include "swissarmynoife/sak_mcp_client.hpp"

#include <httplib.h>
#include <stdexcept>

namespace swissarmynoife {
namespace {

void parse_mcp_url(const std::string& base, std::string& host, int& port, std::string& path) {
  auto u = base;
  while (!u.empty() && u.back() == '/') u.pop_back();
  std::string rest = u;
  if (rest.rfind("https://", 0) == 0) rest = rest.substr(8);
  else if (rest.rfind("http://", 0) == 0) rest = rest.substr(7);
  auto slash = rest.find('/');
  auto authority = slash == std::string::npos ? rest : rest.substr(0, slash);
  path = slash == std::string::npos ? "/" : rest.substr(slash);
  auto colon = authority.rfind(':');
  if (colon != std::string::npos) {
    host = authority.substr(0, colon);
    port = std::stoi(authority.substr(colon + 1));
  } else {
    host = authority;
    port = 80;
  }
}

}  // namespace

SakMcpClient::SakMcpClient(std::string base_url) {
  if (base_url.empty()) base_url = "http://127.0.0.1:8080/mcp";
  parse_mcp_url(base_url, host_, port_, path_);
}

nlohmann::json SakMcpClient::initialize() {
  auto result = rpc("initialize", {
                                    {"protocolVersion", "2024-11-05"},
                                    {"capabilities", nlohmann::json::object()},
                                    {"clientInfo", {{"name", "swissarmynoife-cpp"}, {"version", "0.1.0"}}},
                                });
  httplib::Client cli(host_, port_);
  httplib::Headers headers{{"Content-Type", "application/json"},
                           {"Accept", "application/json, text/event-stream"}};
  if (!token_.empty()) headers.emplace("Authorization", "Bearer " + token_);
  if (session_id_) headers.emplace("mcp-session-id", *session_id_);
  cli.Post(path_, headers, nlohmann::json({{"jsonrpc", "2.0"}, {"method", "notifications/initialized"}}).dump(),
           "application/json");
  initialized_ = true;
  return result;
}

std::string SakMcpClient::ping() { return extract_ping_text(tools_call("ping")); }

nlohmann::json SakMcpClient::tools_list() {
  ensure_session();
  return rpc("tools/list");
}

nlohmann::json SakMcpClient::catalog_list() { return tools_call("catalog_list"); }

void SakMcpClient::ensure_session() {
  if (!auto_initialize_ || initialized_) return;
  initialize();
}

nlohmann::json SakMcpClient::tools_call(const std::string& name, const nlohmann::json& arguments) {
  ensure_session();
  return rpc("tools/call", {{"name", name}, {"arguments", arguments}});
}

nlohmann::json SakMcpClient::rpc(const std::string& method, const nlohmann::json& params) {
  rpc_id_++;
  nlohmann::json payload = {{"jsonrpc", "2.0"}, {"id", rpc_id_}, {"method", method}, {"params", params}};
  httplib::Client cli(host_, port_);
  httplib::Headers headers{{"Content-Type", "application/json"},
                           {"Accept", "application/json, text/event-stream"}};
  if (!token_.empty()) headers.emplace("Authorization", "Bearer " + token_);
  if (session_id_) headers.emplace("mcp-session-id", *session_id_);
  auto res = cli.Post(path_, headers, payload.dump(), "application/json");
  if (!res || res->status < 200 || res->status >= 300) {
    throw std::runtime_error(std::to_string(res ? res->status : 0) + ": " + (res ? res->body : "no response"));
  }
  auto body = nlohmann::json::parse(res->body);
  if (!session_id_) {
    auto it = res->headers.find("mcp-session-id");
    if (it != res->headers.end() && !it->second.empty()) session_id_ = it->second;
  }
  if (!session_id_) session_id_ = session_id_from_body(body);
  if (body.contains("error")) {
    auto err = body["error"];
    auto msg = err.is_object() && err.contains("message") ? err["message"].get<std::string>() : err.dump();
    throw std::runtime_error("MCP " + method + " failed: " + msg);
  }
  return body.contains("result") ? body["result"] : body;
}

std::optional<std::string> SakMcpClient::session_id_from_body(const nlohmann::json& body) {
  for (auto key : {"sessionId", "session_id", "mcp-session-id"}) {
    if (body.contains(key) && body[key].is_string() && !body[key].get<std::string>().empty()) {
      return body[key].get<std::string>();
    }
  }
  if (body.contains("result") && body["result"].is_object()) {
    auto result = body["result"];
    for (auto key : {"sessionId", "session_id", "mcp-session-id"}) {
      if (result.contains(key) && result[key].is_string() && !result[key].get<std::string>().empty()) {
        return result[key].get<std::string>();
      }
    }
  }
  return std::nullopt;
}

std::string SakMcpClient::extract_ping_text(const nlohmann::json& result) {
  if (result.is_string()) return result.get<std::string>();
  if (result.is_object() && result.contains("content") && result["content"].is_array()) {
    for (auto& item : result["content"]) {
      if (item.is_object() && item.contains("text") && item["text"].is_string()) {
        return item["text"].get<std::string>();
      }
    }
  }
  return result.dump();
}

}  // namespace swissarmynoife
