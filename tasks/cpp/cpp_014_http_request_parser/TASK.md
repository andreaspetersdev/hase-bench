# CPP-014 — Incremental HTTP/1.1 request parser

Complete the C++20 implementation declared in `include/http_parser.hpp`.
Do not change the public API, CMake target names, or add external dependencies.

`RequestParser` consumes arbitrary byte chunks through `push`. It is a
deliberately small HTTP/1.1 request parser, not a full HTTP implementation.
Input may split at any byte boundary, including within a CRLF, a header name,
or a body. One call may also contain several pipelined requests. Completed
requests are returned in arrival order by `take_request`.

## Accepted request syntax

Each request is:

```text
METHOD SP request-target SP HTTP/1.1 CRLF
header-name: optional-OWS header-value optional-OWS CRLF
...
CRLF
body
```

- `METHOD` consists of one or more upper-case ASCII letters. `request-target`
  is non-empty, begins with `/`, and consists only of printable ASCII bytes
  other than space. The version is exactly `HTTP/1.1`.
- A header name is one or more HTTP token characters: letters, digits, or one
  of `!#$%&'*+-.^_`|~`. Header names compare case-insensitively, but each
  accepted header retains its original spelling in `HttpRequest::headers`.
- A header value contains printable ASCII bytes or tabs. Leading and trailing
  spaces/tabs are removed; interior spaces/tabs are preserved. Obsolete folded
  headers are not supported.
- Lines must use CRLF exactly. A bare LF or bare CR is malformed.
- `Content-Length`, when present, is a non-empty unsigned base-10 decimal with
  no sign or whitespace. Leading zeroes are allowed. All duplicate
  `Content-Length` fields must have the same *numeric* value; otherwise the
  request is invalid. Without it, the body is empty. The body has exactly that
  many bytes and may contain arbitrary bytes including NUL and CRLF.
- `Transfer-Encoding` is not supported; its presence is an error. Other
  headers are retained, including repeated headers.

## Limits and errors

The constructor limits are positive, otherwise it throws `std::invalid_argument`.
`max_header_bytes` is the maximum number of bytes from the start of a request
through its empty-line CRLF; `max_body_bytes` limits the declared body length.
Exceeding either limit is an error. A partial header which grows beyond the
header limit must fail without waiting for a terminator.


`push` returns `true` while all supplied bytes are accepted. It returns `false`
when the parser reaches an error, records the appropriate `ParserError`, and
enters a permanent error state. Once errored, later `push` calls return false,
do not clear the error, and must not produce further requests. Existing
completed requests remain available to take. `empty()` reports whether no
completed request is waiting; it does not describe partial input.

Use the listed `ParserError` values for the corresponding invalid constructor
configuration, request line, line ending, header syntax, conflicting/invalid
content length, unsupported transfer encoding, oversized header, and oversized
body cases. `ParserError::none` means no error.

Build and run the visible tests before finishing:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
