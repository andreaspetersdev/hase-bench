#pragma once

#include "event.hpp"

#include <cstddef>
#include <string>
#include <string_view>
#include <vector>

class RecordParser {
public:
    explicit RecordParser(std::size_t max_line_bytes) : max_line_bytes_(max_line_bytes) {}
    [[nodiscard]] bool feed(std::string_view chunk, std::vector<Event>& completed);

private:
    std::size_t max_line_bytes_;
    std::string partial_;
};
