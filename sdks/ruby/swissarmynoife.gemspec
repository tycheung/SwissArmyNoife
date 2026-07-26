# frozen_string_literal: true

require_relative "lib/swissarmynoife/version"

Gem::Specification.new do |spec|
  spec.name          = "swissarmynoife"
  spec.version       = SwissArmyNoife::VERSION
  spec.authors       = ["SwissArmyNoife"]
  spec.email         = ["devnull@example.com"]
  spec.summary       = "SwissArmyNoife HTTP admin + MCP clients"
  spec.description   = "Thin Ruby SDK for SwissArmyNoife http-admin and Streamable HTTP MCP (sak333)."
  spec.homepage      = "https://github.com/tycheung/SwissArmyNoife"
  spec.license       = "Apache-2.0"
  spec.required_ruby_version = ">= 3.1.0"
  spec.files = Dir["lib/**/*", "README.md", "examples/**/*"]
  spec.require_paths = ["lib"]
  spec.add_development_dependency "rspec", "~> 3.13"
  spec.add_development_dependency "webmock", "~> 3.23"
  spec.add_development_dependency "rake", "~> 13.0"
end
