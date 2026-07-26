#include <catch2/catch_test_macros.hpp>
#include <string>
#include "swissarmynoife/sdk_info.hpp"

TEST_CASE("sdk info name", "[scaffold]") {
  REQUIRE(std::string(swissarmynoife::SdkInfo::name) == "swissarmynoife-sdk-cpp");
}
