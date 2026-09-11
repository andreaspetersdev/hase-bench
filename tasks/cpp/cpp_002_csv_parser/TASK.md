# CPP-002: CSV parser

Complete `hase::parse_csv` in the supplied C++20 library without changing its public header API.  It parses the entire input into rows and fields.  On success `ParseResult` contains the rows and has an empty error; on failure it has no rows and a non-empty error.

Records are separated by LF (`\n`) or CRLF (`\r\n`).  A final record need not have a line ending.  An empty input has no rows; a blank record has one empty field; and a final record separator does not add another blank record.  Commas separate fields.  Empty leading, middle, and trailing fields are valid.

A field may be quoted only when its first character is `"`.  In a quoted field, commas and physical line endings are field content; CRLF and LF embedded in quoted fields are represented in the resulting string as `\n`.  A bare carriage return within a quoted field is ordinary field content and is preserved as `\r`.  Two consecutive quotes (`""`) represent one literal quote.  After a closing quote, only a comma, a record ending, or end of input is valid.  Reject unterminated quoted fields, quotes in unquoted fields, text after a closing quote, and a bare carriage return outside a quoted field.

This task does not require RFC-perfect CSV support beyond this contract: there is no whitespace trimming, BOM handling, alternate delimiter, character-encoding conversion, or streaming API.  Do not add external dependencies.  Build with CMake and run the visible CTest suite.
