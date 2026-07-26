# frozen_string_literal: true

require "json"
require "net/http"
require "uri"

module SwissArmyNoife
  # Streamable HTTP MCP client (sak333-c).
  class SakMcpClient
    DEFAULT_MCP = "http://127.0.0.1:8080/mcp"
    PROTOCOL = "2024-11-05"
    SESSION_HEADER = "mcp-session-id"
    ACCEPT = "application/json, text/event-stream"

    attr_reader :base_url, :session_id
    attr_accessor :token, :auto_initialize

    def initialize(base_url = DEFAULT_MCP)
      u = (base_url.nil? || base_url.strip.empty?) ? DEFAULT_MCP : base_url
      @base_url = u.sub(%r{/*\z}, "")
      @token = nil
      @auto_initialize = true
      @rpc_id = 0
      @session_id = nil
      @initialized = false
    end

    def initialize_session
      result = rpc("initialize", {
        "protocolVersion" => PROTOCOL,
        "capabilities" => {},
        "clientInfo" => { "name" => "swissarmynoife-ruby", "version" => VERSION }
      })
      post({ "jsonrpc" => "2.0", "method" => "notifications/initialized" }, notification: true)
      @initialized = true
      result
    end

    def ping
      extract_ping_text(tools_call("ping"))
    end

    def tools_list
      ensure_session
      rpc("tools/list")
    end

    def catalog_list
      tools_call("catalog_list")
    end

    private

    def ensure_session
      return if !@auto_initialize || @initialized

      initialize_session
    end

    def tools_call(name, arguments = {})
      ensure_session
      rpc("tools/call", { "name" => name, "arguments" => arguments || {} })
    end

    def rpc(method, params = {})
      @rpc_id += 1
      payload = {
        "jsonrpc" => "2.0",
        "id" => @rpc_id,
        "method" => method,
        "params" => params || {}
      }
      res = post(payload, notification: false)
      body = JSON.parse(res.body)
      capture_session(res, body)
      if body.is_a?(Hash) && body.key?("error")
        err = body["error"]
        msg = err.is_a?(Hash) ? (err["message"] || err) : err
        raise "MCP #{method} failed: #{msg}"
      end
      return body["result"] if body.is_a?(Hash) && body.key?("result")

      body
    end

    def post(payload, notification:)
      uri = URI(@base_url)
      res = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == "https") do |http|
        req = Net::HTTP::Post.new(uri)
        req["Content-Type"] = "application/json"
        req["Accept"] = ACCEPT
        req["Authorization"] = "Bearer #{@token}" if @token && !@token.empty?
        req[SESSION_HEADER] = @session_id if @session_id && !@session_id.empty?
        req.body = JSON.generate(payload)
        http.request(req)
      end
      if notification && %w[200 202].include?(res.code)
        return res
      end
      raise "#{res.code}: #{res.body}" unless res.is_a?(Net::HTTPSuccess)

      res
    end

    def capture_session(res, body)
      if (@session_id.nil? || @session_id.empty?) && res[SESSION_HEADER]
        sid = res[SESSION_HEADER].to_s.strip
        @session_id = sid unless sid.empty?
      end
      return unless @session_id.nil? || @session_id.empty?

      sid = session_id_from_body(body)
      @session_id = sid if sid
    end

    def session_id_from_body(body)
      return nil unless body.is_a?(Hash)

      %w[sessionId session_id mcp-session-id].each do |key|
        v = body[key]
        return v.strip if v.is_a?(String) && !v.strip.empty?
      end
      result = body["result"]
      return nil unless result.is_a?(Hash)

      %w[sessionId session_id mcp-session-id].each do |key|
        v = result[key]
        return v.strip if v.is_a?(String) && !v.strip.empty?
      end
      nil
    end

    def extract_ping_text(result)
      return result if result.is_a?(String)
      return result.to_s unless result.is_a?(Hash)

      content = result["content"]
      if content.is_a?(Array)
        content.each do |item|
          next unless item.is_a?(Hash)

          text = item["text"]
          return text if text.is_a?(String)
        end
      end
      result.to_s
    end
  end
end
