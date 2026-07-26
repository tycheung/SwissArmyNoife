# frozen_string_literal: true

require "spec_helper"

RSpec.describe SwissArmyNoife do
  it "exposes VERSION" do
    expect(SwissArmyNoife::VERSION).to eq("0.1.0")
  end
end
