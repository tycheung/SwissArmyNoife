#pragma once

#include <map>
#include <nlohmann/json.hpp>
#include <string>

namespace swissarmynoife {

/** HTTP admin client (sak336-b). */
class SakClient {
 public:
  explicit SakClient(std::string base_url = "http://127.0.0.1:8787");

  const std::string& base_url() const { return base_url_; }

  nlohmann::json health();
  nlohmann::json list_modules();
  nlohmann::json get_module(const std::string& id);
  nlohmann::json capacity();
  nlohmann::json list_work();
  nlohmann::json list_nodes();
  nlohmann::json compute_work(const nlohmann::json& body);
  nlohmann::json compute_nodes(const nlohmann::json& body);
  nlohmann::json enqueue_work(const std::string& kind, const nlohmann::json& payload = nlohmann::json::object());
  nlohmann::json claim_work(const std::string& node_id);
  nlohmann::json complete_work(const std::string& work_id, const std::string& node_id,
                               const nlohmann::json& result = nlohmann::json::object());
  nlohmann::json get_work(const std::string& work_id);
  nlohmann::json requeue_work(const std::string& work_id);
  nlohmann::json list_work_filtered(const nlohmann::json& filters = nlohmann::json::object());
  nlohmann::json list_nodes_filtered(const nlohmann::json& filters = nlohmann::json::object());

 private:
  nlohmann::json get_json(const std::string& path);
  nlohmann::json post_json(const std::string& path, const nlohmann::json& payload);
  std::string base_url_;
  std::string host_;
  int port_{8787};
  std::string scheme_;
};

}  // namespace swissarmynoife
