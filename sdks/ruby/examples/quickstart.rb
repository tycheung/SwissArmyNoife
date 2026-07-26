# frozen_string_literal: true

require "swissarmynoife"

base = ENV.fetch("SAK_HTTP", "http://127.0.0.1:8787")
sak = SwissArmyNoife::SakClient.new(base)
puts "health=#{sak.health.inspect}"
puts "modules=#{sak.list_modules.inspect}"
