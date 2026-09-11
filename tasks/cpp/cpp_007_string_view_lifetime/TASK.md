# CPP-007 — Template lifetime repair

This small template library has failing tests and sanitizable incorrect
behaviour. Diagnose the problem and repair the implementation without changing
the public headers or CMake target names.

`CompiledTemplate` is an **owning** value type. `compile(std::string)` takes
ownership of its input; a compiled template remains usable after the caller's
string is destroyed. It supports ordinary copy construction, copy assignment,
move construction, and move assignment. A moved-to object remains usable; the
moved-from object's value is unspecified but it must remain destructible and
assignable.

`TemplateView` is deliberately different: it is a non-owning, zero-copy view
created with `from_external`. Its caller must keep the supplied source text
alive and unchanged while the view is used. Do not turn this type into an
owning copy merely to repair the bug.

Both types implement the same tiny template grammar: every exact `{name}`
sequence is replaced by the argument passed to `render`; all other characters,
including unmatched braces and different brace text, are literal text.

`TemplateCache` stores owning compiled templates by key. Inserting an existing
key replaces its template. `find` returns `nullptr` for a missing key, and
`render` throws `std::out_of_range` for a missing key. A pointer returned by
`find` need only remain valid until the next non-const operation on that cache.

You may change implementation files and add private implementation details.
Do not change public function signatures, replace the project with a different
API, or remove the visible tests. Build and run the visible tests before
finishing.
