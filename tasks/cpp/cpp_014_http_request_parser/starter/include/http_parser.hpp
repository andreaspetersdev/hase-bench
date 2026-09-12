#pragma once

#include <cstddef>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

enum class ParserError {
    none,
    invalid_request_line,
    invalid_line_ending,
    invalid_header,
    invalid_content_length,
    conflicting_content_length,
    unsupported_transfer_encoding,
    header_too_large,
    body_too_large,
};

struct Header {
    std::string name;
    std::string value;
};

struct HttpRequest {
    std::string method;
    std::string target;
    std::vector<Header> headers;
    std::string body;

    [[nodiscard]] std::optional<std::string_view> header(std::string_view name) const noexcept;
};

class RequestParser {
public:
    RequestParser(std::size_t max_header_bytes, std::size_t max_body_bytes);

    [[nodiscard]] bool push(std::string_view bytes);
    [[nodiscard]] std::optional<HttpRequest> take_request();
    [[nodiscard]] ParserError error() const noexcept;
    [[nodiscard]] bool empty() const noexcept;

private:
    std::size_t max_header_bytes_;
    std::size_t max_body_bytes_;
    std::string pending_;
    std::vector<HttpRequest> completed_;
    ParserError error_{ParserError::none};
};
