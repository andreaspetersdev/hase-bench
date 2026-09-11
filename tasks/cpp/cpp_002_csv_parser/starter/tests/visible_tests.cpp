#include "csv_parser.hpp"

#include <iostream>
#include <string_view>

namespace {

bool expect_rows(std::string_view input, const hase::CsvRows& expected) {
    const auto result = hase::parse_csv(input);
    return result && *result.rows == expected;
}

bool expect_failure(std::string_view input) {
    const auto result = hase::parse_csv(input);
    return !result && !result.error.empty();
}

}  // namespace

int main() {
    if (!expect_rows("name,age\nAda,37", {{"name", "age"}, {"Ada", "37"}})) {
        std::cerr << "basic rows failed\n";
        return 1;
    }
    if (!expect_rows(",middle,\n\nlast", {{"", "middle", ""}, {""}, {"last"}})) {
        std::cerr << "empty fields failed\n";
        return 2;
    }
    if (!expect_rows("one,\"two, three\",\"say \"\"hello\"\"\"", {{"one", "two, three", "say \"hello\""}})) {
        std::cerr << "quoted fields failed\n";
        return 3;
    }
    if (!expect_failure("one,\"unterminated")) {
        std::cerr << "unterminated quote accepted\n";
        return 4;
    }
}
