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
| CPP-007 | Template lifetime repair | Hard | 1 | Multi-file diagnosis of stale `string_view` state across parser, owning value, cache, and delayed rendering boundaries. Owning templates must have normal value semantics; the distinct externally-backed `TemplateView` must preserve its documented zero-copy semantics. |
| CPP-008 | Deadlock-free account transfers | Hard | 2 | Multi-account transfer repair: local deadlock-free locking under opposing transfers, self/amount/funds result precedence, and exception-safe aggregate-balance invariants even when a throwing transfer overlaps a successful transfer on one account. Hidden validation also proves unrelated account pairs are not globally serialised. |
| CPP-009 | JSON-like configuration merge | Medium | 3 | Precise recursive `variant` merge semantics for objects, arrays (including empty and non-empty whole-array replacement), scalar/type replacement without numeric coercion, absent versus null, deeply nested values, and independent result/input ownership. |
| CPP-010 | Graph dependency resolver | Medium-hard | 1 | Deterministic lexically tie-broken topological resolution; precedence-defined duplicate and missing diagnostics; and canonical closed directed cycle paths across deep, diamond, and disconnected graphs. |
| CPP-011 | QR least-squares solver | Very hard | 2 | Stable overdetermined least-squares solving without normal equations. The explicit dimension/non-finite/rank policy, scale-relative rank threshold based on original columns (including a large later column), noisy/rectangular residual checks, close-column system, and badly scaled full-rank system distinguish practical QR implementations from fragile algebraic shortcuts. |
| CPP-012 | Quaternion rotation utilities | Hard | 1 | Numerically robust right-handed quaternion rotations with explicit non-finite, near-zero, sign-canonicalization, and proper-matrix policies. Independent tests cover composition order, inverse and vector invariants, scale independence, matrix orthogonality/round trip, reflection/skew rejection, and error result values. |
| CPP-013 | Streaming log statistics | Medium-hard | 2 | Strict bounded single-line parsing, inclusive time-window accounting, fixed severity counts, compensated all-history mean, and FIFO percentile sampling with nearest-rank semantics. Independent tests cover input grammar, malformed/outside precedence, sample rolling, empty results, configuration errors, an adversarial large-plus-small mean, and complete long-decimal conversion without truncation. |

## Planned tasks

| ID | Title | Difficulty | Intended benchmark scope |
| --- | --- | --- | --- |
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
