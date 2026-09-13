#pragma once

#include <cstdint>
#include <string>

struct Event {
    std::uint64_t timestamp{};
    std::string source;
    std::uint32_t value{};
};
