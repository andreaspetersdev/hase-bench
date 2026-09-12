#include "http_parser.hpp"

#include <stdexcept>

std::optional<std::string_view> HttpRequest::header(std::string_view) const noexcept {
    return std::nullopt;
}

RequestParser::RequestParser(std::size_t max_header_bytes, std::size_t max_body_bytes)
    : max_header_bytes_(max_header_bytes), max_body_bytes_(max_body_bytes) {
    if (max_header_bytes == 0 || max_body_bytes == 0) throw std::invalid_argument("parser limit must be positive");
}

bool RequestParser::push(std::string_view) {
    return false;
}

std::optional<HttpRequest> RequestParser::take_request() {
    return std::nullopt;
}

ParserError RequestParser::error() const noexcept {
    return error_;
}

bool RequestParser::empty() const noexcept {
    return completed_.empty();
}
