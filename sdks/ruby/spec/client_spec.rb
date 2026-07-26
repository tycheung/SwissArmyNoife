# frozen_string_literal: true

require "spec_helper"
require "webmock/rspec"
require "swissarmynoife"

RSpec.describe SwissArmyNoife::SakClient do
  it "strips trailing slash" do
    c = described_class.new("http://127.0.0.1:8787/")
    expect(c.base_url).to eq("http://127.0.0.1:8787")
  end

  it "health calls endpoint" do
    stub_request(:get, "http://example.test/health").to_return(
      body: '{"ok":true}', headers: { "Content-Type" => "application/json" }
    )
    expect(described_class.new("http://example.test").health).to eq({ "ok" => true })
  end

  {
    list_modules: ["/v1/sak/modules", { "modules" => [] }],
    list_work: ["/v1/sak/compute/work", { "work" => [] }],
    list_nodes: ["/v1/sak/compute/nodes", { "nodes" => [] }],
    capacity: ["/v1/sak/capacity", { "snapshot" => { "total_ram_mb" => 1 } }]
  }.each do |method, (path, body)|
    it "#{method} calls #{path}" do
      stub_request(:get, "http://example.test#{path}").to_return(
        body: JSON.generate(body), headers: { "Content-Type" => "application/json" }
      )
      expect(described_class.new("http://example.test").public_send(method)).to eq(body)
    end
  end

  it "enqueue_work posts action" do
    stub_request(:post, "http://example.test/v1/sak/compute/work")
      .with { |req| JSON.parse(req.body)["action"] == "enqueue" && JSON.parse(req.body)["kind"] == "echo" }
      .to_return(body: '{"action":"enqueue","work":{"status":"queued"}}',
                 headers: { "Content-Type" => "application/json" })
    out = described_class.new("http://example.test").enqueue_work("echo", { "n" => 1 })
    expect(out["action"]).to eq("enqueue")
  end

  it "requeue claim complete get post actions" do
    actions = []
    stub_request(:post, "http://example.test/v1/sak/compute/work").to_return do |req|
      actions << JSON.parse(req.body)["action"]
      { body: '{"action":"ok","work":{"id":"w1"}}', headers: { "Content-Type" => "application/json" } }
    end
    c = described_class.new("http://example.test")
    c.requeue_work("w1")
    c.claim_work("n1")
    c.complete_work("w1", "n1")
    c.get_work("w1")
    expect(actions).to eq(%w[requeue claim complete get])
  end
end
