#pragma once

#include <nlohmann/json.hpp>
#include <optional>
#include <string>

namespace swissarmynoife {

/** Streamable HTTP MCP client (sak336-c). */
class SakMcpClient {
 public:
  explicit SakMcpClient(std::string base_url = "http://127.0.0.1:8080/mcp");

  void set_token(std::string token) { token_ = std::move(token); }
  void set_auto_initialize(bool v) { auto_initialize_ = v; }
  const std::optional<std::string>& session_id() const { return session_id_; }

  nlohmann::json initialize();
  std::string ping();
  nlohmann::json tools_list();
  nlohmann::json catalog_list();

 private:
  void ensure_session();
  nlohmann::json tools_call(const std::string& name,
                            const nlohmann::json& arguments = nlohmann::json::object());
  nlohmann::json rpc(const std::string& method,
                     const nlohmann::json& params = nlohmann::json::object());
  static std::optional<std::string> session_id_from_body(const nlohmann::json& body);
  static std::string extract_ping_text(const nlohmann::json& result);

  std::string host_;
  int port_{8080};
  std::string path_{"/mcp"};
  std::string token_;
  bool auto_initialize_{true};
  int rpc_id_{0};
  std::optional<std::string> session_id_;
  bool initialized_{false};
};

}  // namespace swissarmynoife
