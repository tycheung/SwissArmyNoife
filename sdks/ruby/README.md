# Ruby SDK (`sak333`)

Gem `swissarmynoife` — HTTP admin (`SakClient`) and Streamable HTTP MCP (`SakMcpClient`)
for SwissArmyNoife.

## Test

```bash
cd SwissArmyNoife/sdks/ruby
bundle install
bundle exec rake
```

## Usage

```ruby
require "swissarmynoife"

sak = SwissArmyNoife::SakClient.new("http://127.0.0.1:8787")
health = sak.health
modules = sak.list_modules
```

### MCP

```ruby
mcp = SwissArmyNoife::SakMcpClient.new("http://127.0.0.1:8080/mcp")
mcp.token = ENV["MCP_HTTP_TOKEN"]
pong = mcp.ping
```

Set `auto_initialize = false` in unit tests that mock a single RPC.

## Rails note

In an initializer (e.g. `config/initializers/swissarmynoife.rb`):

```ruby
# frozen_string_literal: true

require "swissarmynoife"

Rails.application.config.swissarmynoife_http =
  ENV.fetch("SAK_HTTP", "http://127.0.0.1:8787")

# Optional helper:
# module Sak
#   def self.client
#     @client ||= SwissArmyNoife::SakClient.new(
#       Rails.application.config.swissarmynoife_http
#     )
#   end
# end
```

Add the gem via path or published source in the app `Gemfile`:

```ruby
gem "swissarmynoife", path: "../SwissArmyNoife/sdks/ruby"
```

## Examples

See [`examples/quickstart.rb`](examples/quickstart.rb):

```bash
cargo run -p http-admin
cd SwissArmyNoife/sdks/ruby
bundle exec ruby examples/quickstart.rb
```

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
