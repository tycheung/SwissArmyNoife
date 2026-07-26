# frozen_string_literal: true

require "spec_helper"
require "webmock/rspec"
require "swissarmynoife"

RSpec.describe SwissArmyNoife::SakMcpClient do
  it "ping negotiates session" do
    stub_request(:post, "http://example.test/mcp").to_return do |req|
      body = JSON.parse(req.body)
      case body["method"]
      when "initialize"
        {
          status: 200,
          headers: { "Content-Type" => "application/json", "mcp-session-id" => "sess-rb-1" },
          body: '{"jsonrpc":"2.0","id":1,"result":{}}'
        }
      when "notifications/initialized"
        { status: 202, body: "" }
      when "tools/call"
        expect(req.headers["Mcp-Session-Id"] || req.headers["mcp-session-id"]).to eq("sess-rb-1")
        {
          status: 200,
          headers: { "Content-Type" => "application/json" },
          body: '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"pong"}]}}'
        }
      else
        raise "unexpected #{body['method']}"
      end
    end
    mcp = described_class.new("http://example.test/mcp")
    expect(mcp.ping).to eq("pong")
    expect(mcp.session_id).to eq("sess-rb-1")
  end

  it "tools_list without auto init" do
    stub_request(:post, "http://example.test/mcp")
      .with { |req| JSON.parse(req.body)["method"] == "tools/list" }
      .to_return(body: '{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}',
                 headers: { "Content-Type" => "application/json" })
    mcp = described_class.new("http://example.test/mcp")
    mcp.auto_initialize = false
    expect(mcp.tools_list).to include("tools" => [])
  end

  it "catalog_list" do
    stub_request(:post, "http://example.test/mcp").to_return do |req|
      body = JSON.parse(req.body)
      case body["method"]
      when "initialize"
        {
          status: 200,
          headers: { "Content-Type" => "application/json", "mcp-session-id" => "s2" },
          body: '{"jsonrpc":"2.0","id":1,"result":{}}'
        }
      when "notifications/initialized"
        { status: 202, body: "" }
      else
        {
          status: 200,
          headers: { "Content-Type" => "application/json" },
          body: '{"jsonrpc":"2.0","id":2,"result":{"offers":[]}}'
        }
      end
    end
    expect(described_class.new("http://example.test/mcp").catalog_list).to include("offers" => [])
  end
end
