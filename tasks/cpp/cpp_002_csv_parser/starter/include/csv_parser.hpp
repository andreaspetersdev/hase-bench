#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace hase {

using CsvRow = std::vector<std::string>;
using CsvRows = std::vector<CsvRow>;

struct ParseResult {
    std::optional<CsvRows> rows;
    std::string error;

    explicit operator bool() const { return rows.has_value(); }
};

ParseResult parse_csv(std::string_view input);

}  // namespace hase
