# frozen_string_literal: true

require "json"
require "net/http"
require "uri"

module SwissArmyNoife
  # HTTP admin client (sak333-b).
  class SakClient
    DEFAULT_HTTP = "http://127.0.0.1:8787"

    attr_reader :base_url

    def initialize(base_url = DEFAULT_HTTP)
      u = (base_url.nil? || base_url.strip.empty?) ? DEFAULT_HTTP : base_url
      @base_url = u.sub(%r{/*\z}, "")
    end

    def health
      get_json("/health")
    end

    def list_modules
      get_json("/v1/sak/modules")
    end

    def get_module(id)
      get_json("/v1/sak/modules/#{URI.encode_www_form_component(id)}")
    end

    def capacity
      get_json("/v1/sak/capacity")
    end

    def list_work
      get_json("/v1/sak/compute/work")
    end

    def list_nodes
      get_json("/v1/sak/compute/nodes")
    end

    def compute_work(body)
      post_json("/v1/sak/compute/work", body)
    end

    def compute_nodes(body)
      post_json("/v1/sak/compute/nodes", body)
    end

    def enqueue_work(kind, payload = {})
      compute_work({ "action" => "enqueue", "kind" => kind, "payload" => payload || {} })
    end

    def claim_work(node_id)
      compute_work({ "action" => "claim", "node_id" => node_id })
    end

    def complete_work(work_id, node_id, result = {})
      compute_work({
        "action" => "complete",
        "work_id" => work_id,
        "node_id" => node_id,
        "result" => result || {}
      })
    end

    def get_work(work_id)
      compute_work({ "action" => "get", "work_id" => work_id })
    end

    def requeue_work(work_id)
      compute_work({ "action" => "requeue", "work_id" => work_id })
    end

    def list_work_filtered(filters = {})
      compute_work({ "action" => "list" }.merge(filters || {}))
    end

    def list_nodes_filtered(filters = {})
      compute_nodes({ "action" => "list" }.merge(filters || {}))
    end

    def register_node(label, caps: nil, node_id: nil, session_id: nil)
      body = { "action" => "register", "label" => label }
      body["caps"] = caps if caps
      body["node_id"] = node_id if node_id && !node_id.empty?
      body["session_id"] = session_id if session_id && !session_id.empty?
      compute_nodes(body)
    end

    def heartbeat_node(node_id)
      compute_nodes({ "action" => "heartbeat", "node_id" => node_id })
    end

    private

    def get_json(path)
      uri = URI("#{@base_url}#{path}")
      res = Net::HTTP.get_response(uri)
      raise "#{res.code}: #{res.body}" unless res.is_a?(Net::HTTPSuccess)

      JSON.parse(res.body)
    end

    def post_json(path, payload)
      uri = URI("#{@base_url}#{path}")
      res = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == "https") do |http|
        req = Net::HTTP::Post.new(uri)
        req["Content-Type"] = "application/json"
        req.body = JSON.generate(payload)
        http.request(req)
      end
      raise "#{res.code}: #{res.body}" unless res.is_a?(Net::HTTPSuccess)

      JSON.parse(res.body)
    end
  end
end
