# Hase Bench task catalogue

This is the maintained human-readable catalogue for the C++ suite.  Canonical
agent-visible requirements live in each implemented task's `TASK.md`; this
file explains the intended scope and difficulty balance.  Update it with
`CODEX_TASK.md`, the task metadata, and `progress.md` whenever a task or its
version changes.

## Implemented tasks

| ID | Title | Difficulty | Version | Scope and review focus |
| --- | --- | --- | --- | --- |
| CPP-001 | Expression evaluator | Medium | 2 | Recursive-descent arithmetic parser: precedence, unary operators, decimal syntax, `std::isspace`, and explicit malformed-input/division-by-zero behavior. Scientific notation remains outside scope. |
| CPP-002 | CSV parser | Medium | 2 | Bounded whole-input CSV state machine: quotes, escaped quotes, embedded newline normalization, CRLF/LF, final records, and malformed quoting. It deliberately does not become an RFC-complete or streaming parser. |
| CPP-003 | Generic LRU cache | Medium | 2 | Generic O(1)-expected LRU semantics, capacity edge cases, rvalue keys, and move-only values. Copy/move and stronger pointer validity are explicitly outside its public contract. |
| CPP-004 | Thread pool | Hard | 3 | Fixed-worker C++20 pool with futures, exceptions, concurrent submission, forwarding of lvalue/rvalue and move-only work, draining shutdown, and destructor draining. It intentionally excludes concurrent `shutdown()` calls and shutdown from a worker, keeping the challenge focused on a reliable conventional pool. |
| CPP-005 | SPSC ring buffer | Very hard | 3 | Lock-free single-producer/single-consumer queue with exact capacity, `Capacity == 1`, wrap-around, move-only/raw-storage lifetime correctness, failed-pop preservation, and acquire/release payload publication. The hidden suite uses a deterministic structured-payload stress test plus a narrow source-contract check for the explicit no-mutex/non-`seq_cst`-only requirement. |
| CPP-006 | Binary serialization | Medium-hard | 2 | Exact fixed big-endian packet encoding with bounded payloads, FNV-1a integrity checking, and non-overlapping invalid-header, impossible-length, truncation, and corruption classifications, including explicit partial-magic precedence. |

## Planned tasks

| ID | Title | Difficulty | Intended benchmark scope |
| --- | --- | --- | --- |
| CPP-007 | Existing bug: lifetime / `string_view` | Hard | At least three files and two ownership boundaries conceal a dangling-view defect; the repair must preserve the public API, valid external zero-copy use, and correct copy/move behavior. |
| CPP-008 | Existing bug: deadlock | Hard | Barrier-driven opposing transfers, self/insufficient-resource/error paths, aggregate-balance invariants, and a real local lock-order strategy rather than global locking or sleep retries. |
| CPP-009 | JSON-like configuration merge | Medium | Precise recursive `variant` merge semantics for objects, arrays, scalar/type replacement, absent versus null, deeply nested values, and non-aliasing inputs. |
| CPP-010 | Graph dependency resolver | Medium-hard | Deterministic lexically tie-broken topological resolution, duplicate/missing diagnostics, and closed useful cycle paths across deep and disconnected graphs. |
| CPP-011 | Matrix / least squares | Very hard | QR-based solver with dimension/rank/non-finite policy, residual-quality validation, rectangular/noisy/badly-scaled systems, and tests that expose fragile normal equations. |
| CPP-012 | Geometry / quaternion | Hard | Rotation operations with explicit zero normalization, sign ambiguity, invalid-matrix policy, and invariant-based composition/orthogonality/round-trip validation. |
| CPP-013 | Log parser and statistics | Medium-hard | Bounded-memory aggregation, documented percentile window, malformed-record accounting, timestamp boundary policy, large streams, and numeric-stability checks. |
| CPP-014 | HTTP request parser | Hard | Incremental HTTP/1.1 parsing at every byte split, bounded headers/bodies, duplicate-length/error policy, pipelined remainders, and an irreversible malformed-input state. |
| CPP-015 | Generic event dispatcher | Hard | Type-safe events with deterministic re-entrant subscribe/unsubscribe semantics, move-safe idempotent tokens, and explicit exception isolation/propagation policy. |
| CPP-016 | Resource pool with RAII handle | Hard | Ownership repair across nested scopes, exceptions, pool/handle moves, destruction races as specified, resource reuse, and double-return prevention. |
| CPP-017 | Refactor legacy polymorphism | Very hard | Behavior-preserving multi-file refactor where a new feature creates a new behavioral axis; legacy factories/configuration remain compatible and validation stays architecture-neutral. |
| CPP-018 | Image processing kernel | Medium-hard | Stride- and aliasing-aware image processing with deterministic borders, dimension rules, precision/clamping, tiny images, and trusted-reference checks. |
| CPP-019 | Concurrent message recorder | Very hard | Controlled burst/full-queue/writer-failure/shutdown scenarios, specified backpressure, monotonic sequencing, exactly-once persistence of accepted messages, and clean teardown. |
| CPP-020 | Integrated mini service | Expert | At least four interacting components with lifetime and shutdown/error defects plus a feature crossing parser/configuration/queue/statistics; hidden lifecycle, rollback, integration, and legacy-compatibility checks. |
| CPP-021 | SIMD PCA / covariance kernel | Expert | Stride-aware PCA with deterministic symmetric eigensolving, degeneracy/non-finite policy, scalar correctness, optional runtime-dispatched AVX2 accumulation, forced backend validation, and projection/reconstruction residual checks. |

## Difficulty policy

- **Medium** tasks isolate one primary implementation skill and have a narrow, explicit contract.
- **Medium-hard** tasks combine that skill with robust input/error handling or algorithmic depth.
- **Hard** tasks require more than a happy-path implementation: realistic API ownership/forwarding, multi-threaded or multi-file behavior, and independent regression coverage, while avoiding unnecessary platform dependence.
- **Very hard** tasks demand a substantive correctness model, not merely higher loop counts. Their contracts must state the relevant lifetime, synchronization, numerical, or integration invariants, and their hidden tests must exercise those invariants deterministically.
- **Expert** tasks combine several independently testable components and require repository-level diagnosis; they must still have objective validation.
