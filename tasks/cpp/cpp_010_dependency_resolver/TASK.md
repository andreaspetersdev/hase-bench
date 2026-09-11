# CPP-010 — Graph dependency resolver

Complete the C++20 dependency resolver declared in
`include/dependencies/resolver.hpp`. Do not change its public API or the CMake
target names, and do not add external dependencies.

Each `Module` has a unique non-empty `name`. Its `dependencies` are names of
modules that must be resolved before that module. `resolve` must either return
a deterministic dependency order or one precise diagnostic.
Repeated occurrences of the same dependency in one module's list represent
one dependency edge.

On success, `error` is `ResolutionError::none`, `order` contains every module
exactly once, and every dependency appears before its dependent. If several
modules are available at the same point, choose the lexicographically smallest
name. The result must therefore not depend on the input vector order or the
order in a module's `dependencies` vector.

The resolver detects errors in this order:

1. Duplicate definitions: return `duplicate_module`. `module` is the
   lexicographically smallest name defined more than once.
2. Missing dependencies: return `missing_dependency`. Choose the
   lexicographically smallest dependent module that has a missing dependency;
   `module` is that dependent and `dependency` is its lexicographically
   smallest missing dependency.
3. Cycles: return `cycle`. Search module names and each dependency list in
   lexicographic order, stopping at the first DFS back-edge. `cycle_path` must
   list the directed cycle induced by that back-edge, with its first node
   repeated at the end. Rotate it so its first (and final) node is the
   lexicographically smallest name in that cycle. `module`, `dependency`, and
   `order` are empty for a cycle result.

For every failure, `order` is empty. For duplicate or missing diagnostics,
`cycle_path` is empty. For duplicate diagnostics, `dependency` is empty; for
missing diagnostics, `cycle_path` is empty.

An empty input is a successful empty order. Module names are ordinary
case-sensitive strings; this task does not require package versions, optional
dependencies, incremental updates, or parallel resolution.

Build and run the visible tests before finishing:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
