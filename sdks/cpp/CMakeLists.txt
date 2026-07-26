cmake_minimum_required(VERSION 3.20)
project(swissarmynoife_sdk VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(swissarmynoife_sdk
  src/sdk_info.cpp
)
target_include_directories(swissarmynoife_sdk PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
)

include(FetchContent)
FetchContent_Declare(
  Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG v3.7.1
)
FetchContent_MakeAvailable(Catch2)

add_executable(swissarmynoife_sdk_tests
  tests/scaffold_test.cpp
)
target_link_libraries(swissarmynoife_sdk_tests PRIVATE swissarmynoife_sdk Catch2::Catch2WithMain)

include(CTest)
include(Catch)
catch_discover_tests(swissarmynoife_sdk_tests)
