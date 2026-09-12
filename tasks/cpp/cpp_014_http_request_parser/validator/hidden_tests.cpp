#include "http_parser.hpp"

#include <cassert>
#include <stdexcept>
#include <string>

namespace {

void check_every_split(std::string request) {
    for (std::size_t split = 0; split <= request.size(); ++split) {
        RequestParser parser(512, 64);
        assert(parser.push(std::string_view(request).substr(0, split)));
        assert(parser.push(std::string_view(request).substr(split)));
        auto parsed = parser.take_request();
        assert(parsed && parsed->method == "POST" && parsed->target == "/a?x=1");
        assert(parsed->body == std::string("a\0b", 3));
        assert(parsed->header("x-META") == "one\ttwo");
        assert(parsed->header("missing") == std::nullopt);
        assert(parser.empty() && parser.error() == ParserError::none);
    }
}

void check_error(std::string_view input, ParserError expected) {
    RequestParser parser(128, 16);
    assert(!parser.push(input));
    assert(parser.error() == expected);
    assert(!parser.push("GET / HTTP/1.1\r\n\r\n"));
    assert(parser.error() == expected);
    assert(!parser.take_request());
}

} // namespace

int main() {
    bool threw = false;
    try { RequestParser bad(0, 1); } catch (const std::invalid_argument&) { threw = true; }
    assert(threw);
    threw = false;
    try { RequestParser bad(1, 0); } catch (const std::invalid_argument&) { threw = true; }
    assert(threw);

    check_every_split(std::string("POST /a?x=1 HTTP/1.1\r\nHost: x\r\nX-Meta:  one\ttwo \r\nContent-Length: 3\r\n\r\na") + std::string("\0b", 2));

    RequestParser crlf_boundary(128, 8);
    assert(crlf_boundary.push("GET / HTTP/1.1\r"));
    assert(crlf_boundary.push("\n\r\n"));
    assert(crlf_boundary.take_request());
    RequestParser bare_cr_boundary(128, 8);
    assert(bare_cr_boundary.push("GET / HTTP/1.1\r"));
    assert(!bare_cr_boundary.push("X"));
    assert(bare_cr_boundary.error() == ParserError::invalid_line_ending);

    RequestParser pipeline(256, 8);
    assert(pipeline.push("GET /one HTTP/1.1\r\nX: y\r\n\r\nGET /two HTTP/1.1\r\n\r\n"));
    auto one = pipeline.take_request();
    auto two = pipeline.take_request();
    assert(one && two && one->target == "/one" && two->target == "/two" && pipeline.empty());

    check_error("get / HTTP/1.1\r\n\r\n", ParserError::invalid_request_line);
    check_error("GET / HTTP/1.0\r\n\r\n", ParserError::invalid_request_line);
    check_error("GET / HTTP/1.1\n\n", ParserError::invalid_line_ending);
    check_error("GET / HTTP/1.1\rX", ParserError::invalid_line_ending);
    check_error("GET / HTTP/1.1\r\n bad: x\r\n\r\n", ParserError::invalid_header);
    check_error("GET / HTTP/1.1\r\nNo Space: x\r\n\r\n", ParserError::invalid_header);
    check_error("GET / HTTP/1.1\r\nContent-Length: +1\r\n\r\n", ParserError::invalid_content_length);
    check_error("GET / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nx", ParserError::conflicting_content_length);
    check_error("GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n", ParserError::unsupported_transfer_encoding);

    RequestParser body_limit(128, 2);
    assert(!body_limit.push("POST / HTTP/1.1\r\nContent-Length: 3\r\n\r\n"));
    assert(body_limit.error() == ParserError::body_too_large);
    RequestParser header_limit(20, 1);
    assert(!header_limit.push("GET / HTTP/1.1\r\nLong: x"));
    assert(header_limit.error() == ParserError::header_too_large);
}
