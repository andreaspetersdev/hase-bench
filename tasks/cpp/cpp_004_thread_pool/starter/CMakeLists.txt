cmake_minimum_required(VERSION 3.20)
project(cpp_004 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(thread_pool INTERFACE)
target_include_directories(thread_pool INTERFACE include)

enable_testing()
add_executable(thread_pool_visible tests/visible_tests.cpp)
target_link_libraries(thread_pool_visible PRIVATE thread_pool)
add_test(NAME thread_pool_visible COMMAND thread_pool_visible)
