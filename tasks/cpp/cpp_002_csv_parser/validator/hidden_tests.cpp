#include "csv_parser.hpp"

#include <iostream>
#include <string_view>

namespace {

bool expect_rows(std::string_view input, const hase::CsvRows& expected) {
    const auto result = hase::parse_csv(input);
    return result && result.error.empty() && *result.rows == expected;
}

bool expect_failure(std::string_view input) {
    const auto result = hase::parse_csv(input);
    return !result && !result.error.empty();
}

}  // namespace

int main() {
    if (!expect_rows("a,b\r\nc,d\r\n", {{"a", "b"}, {"c", "d"}})) {
        std::cerr << "CRLF or final separator handling failed\n";
        return 1;
    }
    if (!expect_rows("\"first\nsecond\",tail\n\"a\r\nb\",x", {{"first\nsecond", "tail"}, {"a\nb", "x"}})) {
        std::cerr << "embedded newline handling failed\n";
        return 2;
    }
    if (!expect_rows("\"left\rright\",tail", {{"left\rright", "tail"}})) {
        std::cerr << "quoted bare carriage return handling failed\n";
        return 3;
    }
    if (!expect_rows("\"\"\n,\n\"comma, and \"\"quote\"\"\"", {{""}, {"", ""}, {"comma, and \"quote\""}})) {
        std::cerr << "empty or escaped quoted fields failed\n";
        return 4;
    }
    for (const auto input : {"a\"b", "\"a\"x", "\"a\" \n", "a\rb", "\"a\"\"b"}) {
        if (!expect_failure(input)) {
            std::cerr << "accepted malformed input: " << input << '\n';
            return 5;
        }
    }
}
