# CPP-001: Expression evaluator

Implement `hase::evaluate` in the supplied arithmetic library. The project uses C++20.

Expressions support decimal floating-point literals in the forms `12`, `12.`, `.5`, and `12.5`; whitespace as recognised by C++ `std::isspace`; binary `+`, `-`, `*`, `/`; parentheses; and unary `+`/`-`. Usual precedence applies, and binary operators associate left-to-right. `Evaluation` must contain a value on success and a non-empty error message on failure.

Reject malformed numeric literals (including a bare decimal point or multiple decimal points), malformed input, empty input, trailing tokens, unmatched parentheses, and division by zero (including signed zero). Scientific/exponent notation such as `1e3`, locale-specific formats, and non-finite literals are outside this benchmark's scope; their behaviour is not scored. Do not add external dependencies or change the public header API. Build with CMake and run the visible CTest suite.
