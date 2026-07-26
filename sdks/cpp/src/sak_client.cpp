#include "swissarmynoife/sak_client.hpp"

#include <httplib.h>
#include <stdexcept>

namespace swissarmynoife {
namespace {

void parse_base(const std::string& base, std::string& scheme, std::string& host, int& port) {
  auto u = base;
  while (!u.empty() && u.back() == '/') u.pop_back();
  scheme = "http";
  std::string rest = u;
  if (rest.rfind("https://", 0) == 0) {
    scheme = "https";
    rest = rest.substr(8);
  } else if (rest.rfind("http://", 0) == 0) {
    rest = rest.substr(7);
  }
  auto slash = rest.find('/');
  auto authority = slash == std::string::npos ? rest : rest.substr(0, slash);
  auto colon = authority.rfind(':');
  if (colon != std::string::npos) {
    host = authority.substr(0, colon);
    port = std::stoi(authority.substr(colon + 1));
  } else {
    host = authority;
    port = scheme == "https" ? 443 : 80;
  }
}

}  // namespace

SakClient::SakClient(std::string base_url) {
  while (!base_url.empty() && base_url.back() == '/') base_url.pop_back();
  if (base_url.empty()) base_url = "http://127.0.0.1:8787";
  base_url_ = base_url;
  parse_base(base_url_, scheme_, host_, port_);
}

nlohmann::json SakClient::health() { return get_json("/health"); }
nlohmann::json SakClient::list_modules() { return get_json("/v1/sak/modules"); }
nlohmann::json SakClient::get_module(const std::string& id) {
  return get_json("/v1/sak/modules/" + id);
}
nlohmann::json SakClient::capacity() { return get_json("/v1/sak/capacity"); }
nlohmann::json SakClient::list_work() { return get_json("/v1/sak/compute/work"); }
nlohmann::json SakClient::list_nodes() { return get_json("/v1/sak/compute/nodes"); }

nlohmann::json SakClient::compute_work(const nlohmann::json& body) {
  return post_json("/v1/sak/compute/work", body);
}
nlohmann::json SakClient::compute_nodes(const nlohmann::json& body) {
  return post_json("/v1/sak/compute/nodes", body);
}

nlohmann::json SakClient::enqueue_work(const std::string& kind, const nlohmann::json& payload) {
  return compute_work({{"action", "enqueue"}, {"kind", kind}, {"payload", payload}});
}
nlohmann::json SakClient::claim_work(const std::string& node_id) {
  return compute_work({{"action", "claim"}, {"node_id", node_id}});
}
nlohmann::json SakClient::complete_work(const std::string& work_id, const std::string& node_id,
                                        const nlohmann::json& result) {
  return compute_work(
      {{"action", "complete"}, {"work_id", work_id}, {"node_id", node_id}, {"result", result}});
}
nlohmann::json SakClient::get_work(const std::string& work_id) {
  return compute_work({{"action", "get"}, {"work_id", work_id}});
}
nlohmann::json SakClient::requeue_work(const std::string& work_id) {
  return compute_work({{"action", "requeue"}, {"work_id", work_id}});
}
nlohmann::json SakClient::list_work_filtered(const nlohmann::json& filters) {
  auto body = filters;
  body["action"] = "list";
  return compute_work(body);
}
nlohmann::json SakClient::list_nodes_filtered(const nlohmann::json& filters) {
  auto body = filters;
  body["action"] = "list";
  return compute_nodes(body);
}

nlohmann::json SakClient::get_json(const std::string& path) {
  httplib::Client cli(host_, port_);
  cli.set_connection_timeout(5, 0);
  cli.set_read_timeout(30, 0);
  auto res = cli.Get(path);
  if (!res || res->status < 200 || res->status >= 300) {
    throw std::runtime_error(std::to_string(res ? res->status : 0) + ": " + (res ? res->body : "no response"));
  }
  return nlohmann::json::parse(res->body);
}

nlohmann::json SakClient::post_json(const std::string& path, const nlohmann::json& payload) {
  httplib::Client cli(host_, port_);
  cli.set_connection_timeout(5, 0);
  cli.set_read_timeout(30, 0);
  auto res = cli.Post(path, payload.dump(), "application/json");
  if (!res || res->status < 200 || res->status >= 300) {
    throw std::runtime_error(std::to_string(res ? res->status : 0) + ": " + (res ? res->body : "no response"));
  }
  return nlohmann::json::parse(res->body);
}

}  // namespace swissarmynoife
