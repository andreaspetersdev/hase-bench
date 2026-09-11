cmake_minimum_required(VERSION 3.20)
project(cpp_004_hidden LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT STARTER_DIR)
  message(FATAL_ERROR "STARTER_DIR is required")
endif()

add_subdirectory("${STARTER_DIR}" starter)
add_executable(thread_pool_hidden hidden_tests.cpp)
target_link_libraries(thread_pool_hidden PRIVATE thread_pool)

enable_testing()
add_test(NAME thread_pool_hidden COMMAND thread_pool_hidden)
set_tests_properties(thread_pool_hidden PROPERTIES TIMEOUT 30)
