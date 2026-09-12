# CPP-017 — Refactor legacy polymorphism

This C++20 project renders audit records through a legacy class hierarchy.
Implement the `redact` behavior axis requested in `include/audit.hpp`, without
breaking the existing `text` and `json` factory/configuration forms. A redacted
record replaces every value whose key is in the supplied sensitive-key set with
`"***"`; keys, ordering, escaping, and all non-sensitive values remain exact.

Do not change the public API or add dependencies. `make_renderer` must reject
unknown formats with `std::invalid_argument`. Keep the existing output formats
and make redaction work for both formats without duplicating renderer classes
for every format/redaction combination. Empty sensitive-key sets preserve legacy
output exactly. Run the visible CMake tests before finishing.
