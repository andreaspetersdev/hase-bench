# CPP-001: Expression evaluator

Implement `hase::evaluate` in the supplied arithmetic library. The project uses C++20.

Expressions support decimal floating-point literals, whitespace, binary `+`, `-`, `*`, `/`, parentheses, and unary `+`/`-`. Usual precedence applies, and binary operators associate left-to-right. `Evaluation` must contain a value on success and a non-empty error message on failure.

Reject malformed input, empty input, trailing tokens, unmatched parentheses, and division by zero. Do not add external dependencies or change the public header API. Build with CMake and run the visible CTest suite.
