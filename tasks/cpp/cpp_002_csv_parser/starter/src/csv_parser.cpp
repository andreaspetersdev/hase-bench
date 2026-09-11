#include "csv_parser.hpp"

#include <sstream>

namespace hase {

ParseResult parse_csv(std::string_view input) {
    CsvRows rows;
    std::istringstream stream{std::string(input)};
    std::string line;
    while (std::getline(stream, line)) {
        CsvRow row;
        std::istringstream fields(line);
        std::string field;
        while (std::getline(fields, field, ',')) {
            row.push_back(field);
        }
        rows.push_back(std::move(row));
    }
    return {std::move(rows), {}};
}

}  // namespace hase
