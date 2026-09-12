#include "http_parser.hpp"

#include <cassert>

int main() {
    RequestParser parser(256, 64);
    assert(parser.push("GET /status HTTP/1.1\r\nHost: example.test\r\nX-Flag:  yes \t\r\n\r\n"));
    auto request = parser.take_request();
    assert(request && request->method == "GET" && request->target == "/status");
    assert(request->body.empty() && request->headers.size() == 2);
    assert(request->header("host") == "example.test");
    assert(request->header("X-FLAG") == "yes");
    assert(parser.empty() && parser.error() == ParserError::none);

    assert(parser.push("POST /items HTTP/1.1\r\ncontent-length: 5\r\nContent-Length: 005\r\n\r\nhe"));
    assert(!parser.take_request());
    assert(parser.push("lloGET /next HTTP/1.1\r\n\r\n"));
    auto post = parser.take_request();
    auto next = parser.take_request();
    assert(post && post->body == "hello");
    assert(next && next->method == "GET" && next->target == "/next");

    RequestParser malformed(128, 8);
    assert(!malformed.push("GET / HTTP/1.1\n\n"));
    assert(malformed.error() == ParserError::invalid_line_ending);
    assert(!malformed.push("GET /later HTTP/1.1\r\n\r\n"));
}
