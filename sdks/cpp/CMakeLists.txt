cmake_minimum_required(VERSION 3.20)
project(swissarmynoife_sdk VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
FetchContent_Declare(
  httplib
  GIT_REPOSITORY https://github.com/yhirose/cpp-httplib.git
  GIT_TAG v0.18.3
)
FetchContent_Declare(
  json
  GIT_REPOSITORY https://github.com/nlohmann/json.git
  GIT_TAG v3.11.3
)
FetchContent_Declare(
  Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG v3.7.1
)
FetchContent_MakeAvailable(httplib json Catch2)

add_library(swissarmynoife_sdk
  src/sdk_info.cpp
  src/sak_client.cpp
  src/sak_mcp_client.cpp
)
target_include_directories(swissarmynoife_sdk PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
)
target_link_libraries(swissarmynoife_sdk PUBLIC httplib::httplib nlohmann_json::nlohmann_json)
if(WIN32)
  target_link_libraries(swissarmynoife_sdk PUBLIC ws2_32)
endif()

add_executable(swissarmynoife_sdk_tests
  tests/scaffold_test.cpp
  tests/sak_client_test.cpp
  tests/sak_mcp_client_test.cpp
)
target_link_libraries(swissarmynoife_sdk_tests PRIVATE swissarmynoife_sdk Catch2::Catch2WithMain)

include(CTest)
include(Catch)
catch_discover_tests(swissarmynoife_sdk_tests)
