# Hase Bench task catalogue

This is the maintained human-readable catalogue for the C++ suite and planned
Rust tasks. Canonical
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
| CPP-014 | HTTP request parser | Hard | 3 | Bounded incremental HTTP/1.1 request parser: strict CRLF/message grammar, case-insensitive lookup with preserved field spelling, Content-Length duplicate policy, arbitrary body bytes, request pipelining, and an irreversible malformed-input state. Hidden validation replays every split point, checks CRLF boundaries, rejects decimal overflow, and preserves completed requests after a later pipeline error. |
| CPP-015 | Generic event dispatcher | Hard | 1 | Type-safe erased callback dispatch with deterministic subscription order, snapshot delivery under re-entrant removal/addition, nested emits, exception propagation, event-type isolation, and move-safe idempotent subscription tokens. |
| CPP-016 | Resource pool with RAII handle | Hard | 1 | RAII ownership and move-semantics repair: exact-once return, exhaustion/reuse, handle move assignment, pool move construction/assignment, stale-handle inertness, and safe destruction boundaries. |
| CPP-017 | Refactor legacy polymorphism | Very hard | 1 | Behavior-preserving legacy text/JSON renderer refactor: add redaction as a cross-cutting behavior axis while retaining factory compatibility, ordering, escaping, repeated keys, and invalid-format handling. |
| CPP-018 | Image processing kernel | Medium-hard | 1 | Stride-aware grayscale 3×3 valid-neighbor box blur with exact floor rounding, deterministic borders, padding preservation, dimension checks, and active-range alias rejection. |
| CPP-019 | Concurrent message recorder | Very hard | 2 | Bounded multi-producer timestamped-byte recorder with a dedicated sink writer, immediate full-queue rejection, gap-free accepted sequence numbers, drain-on-close, and explicit failed/queued message recovery. Controlled sink gates check full-queue, failure, and shutdown behavior; version 2 also checks the values delivered to the sink and failure visibility before close. |
| CPP-020 | Integrated mini service | Expert | 1 | Five-file C++20 source-aware sensor service: owned configuration, legacy/tagged chunked parsing, transactional batch admission, bounded sink worker, per-source alert statistics, failure recovery, and drain-on-close. Separate hidden component and integration targets check ownership, every split point, rollback/retry, queue capacity, shutdown, and sink failure. |
| CPP-021 | SIMD PCA / covariance kernel | Expert | 2 | Stride-aware C++20 PCA for up to eight features: population covariance, ordered symmetric eigensystem, sign-canonical axes, projection/reconstruction, finite-input policy, and runtime-dispatched scalar/AVX2 covariance paths. Independent hidden checks cover dimensions, padding, full feature width, degeneracy, dispatch, numerical residuals, and tiny but normal covariance scales without absolute eigensolver cutoffs. |

## Planned tasks

No further C++ tasks are currently planned.

### Rust

Rust uses a four-level planning severity scale: **Low**, **Medium**, **High**,
and **Very high**. When a planned task is implemented, these map to the
framework's existing `difficulty` metadata values `easy`, `medium`, `hard`, and
`very hard`. The labels measure interacting correctness obligations, not code
volume or test count.

| ID | Title | Severity | Status | Scope and review focus |
| --- | --- | --- | --- | --- |
| RUST-001 (`rust_001`) | Owned configuration snapshot repair | Medium | Implemented (v2) | Rust 2024 ownership/lifetime repair. The starter's behavior works while the source remains alive, but its public snapshot borrows parsed text. The required result owns entries, accepts owned updates, and clones independently. Hidden compilation and runtime checks enforce source independence, ownership transfer, required public traits, ASCII-only key folding, preserved ordering, and parsing edge cases. Version 2 closes trait and ASCII-boundary coverage gaps found during the first autonomous review without changing the agent-visible contract. |
| RUST-002 (`rust_002`) | Lazy generic merge-join iterator | Medium | Implemented (v1) | Rust 2024 generic iterator implementation. Merge two differently typed sorted streams lazily into left/right/both outputs without `Clone` or `Ord` item bounds. Hidden tests cover move-only values, required output traits, stateful comparators, duplicate pairing, one-item lookahead, comparator exhaustion, complete `size_hint` bounds, and conditional `FusedIterator`. |
| RUST-003 (`rust_003`) | Context-rich operation pipeline | Low | Implemented (v1) | Focused Rust 2024 error-propagation task. Parse and apply checked updates to existing counters in sequence, preserving zero-based failure context, typed `From` conversions, and a two-level `Error::source` chain. Hidden tests cover public traits, the explicit parse/lookup/overflow precedence matrix, ASCII-horizontal trimming, case-sensitive keys, checked limits, partial progress, empty input, and owned insertion. Its narrowness deliberately provides an accessible Rust task. |
| RUST-004 (`rust_004`) | Bounded channel worker | High | Planned | Multi-producer worker owning a dedicated thread and bounded channel. Define admission/backpressure, move-only jobs, ordered accepted work, result delivery, drain-on-close, repeated close, worker failure, and recovery of accepted-but-unfinished jobs. Use gates/barriers rather than timing sleeps to validate full queues, shutdown, and failure deterministically. |
| RUST-005 (`rust_005`) | Incremental binary frame parser | High | Planned | Chunked state machine for a length-prefixed binary protocol with split headers/payloads, multiple frames per chunk, hard size limits, arbitrary bytes, explicit error precedence, completed-frame preservation, and an irreversible failed state. Hidden tests should replay every split point and adversarial lengths without relying on external crates. |
| RUST-006 (`rust_006`) | Trait-driven storage refactor | High | Planned | Multi-file architecture/refactoring task. Replace a closed concrete backend with an object-safe storage/transaction interface while preserving legacy behavior, typed errors, deterministic middleware order, rollback, and caller-owned data. Hidden tests should inject independent backends and failures without downcasting or task-specific hooks. |
| RUST-007 (`rust_007`) | Concurrent single-flight cache | Very high | Planned | Sharded bounded cache with caller-supplied clock, TTL, per-key single-flight loading, LRU-style eviction, and no global serialization of unrelated keys. Specify panic/error wake-up behavior, re-entrant loader limits, exact capacity accounting, and shutdown. Hidden validation needs controlled loaders and clocks to prove waiter progress, failure cleanup, eviction, and disjoint-key concurrency deterministically. |
| RUST-008 (`rust_008`) | Cancellation-safe async service | Very high | Planned | Pinned-runtime async service combining bounded admission, request multiplexing, per-stream ordering, cancellation, deadlines through an injected clock, graceful shutdown, and transport failure propagation. Validation should use a deterministic fake transport/clock, paused time, and controlled tasks; no wall-clock sleeps or network access. Cancellation must not leak permits, lose acknowledged work, or deadlock shutdown. |
| RUST-RSYNC (`rust_rsync`) | Cross-platform rsync clone | Very high / long horizon | Planned | Capstone repository task implementing a full functional rsync clone for Windows and Linux. Pin an upstream rsync release and its official man pages when authoring the task; cover local, remote-shell, and daemon transfers, wire interoperability, delta updates, filters, deletion, metadata, resume, diagnostics, and exit behavior. Build only after the compact High and Very-high tasks validate the Rust workflow, using the gates in [RUST_RSYNC_PLAN.md](RUST_RSYNC_PLAN.md). |

The Rust distribution is one implemented Low task, two implemented Medium
tasks, three planned High tasks, and three planned Very-high tasks including
the rsync capstone. RUST-004 through
RUST-008 are planning entries only; their IDs reserve ordering but do not imply
that implementation may skip contract design, reference review, or independent
validator review.

## C++ difficulty policy

- **Medium** tasks isolate one primary implementation skill and have a narrow, explicit contract.
- **Medium-hard** tasks combine that skill with robust input/error handling or algorithmic depth.
- **Hard** tasks require more than a happy-path implementation: realistic API ownership/forwarding, multi-threaded or multi-file behavior, and independent regression coverage, while avoiding unnecessary platform dependence.
- **Very hard** tasks demand a substantive correctness model, not merely higher loop counts. Their contracts must state the relevant lifetime, synchronization, numerical, or integration invariants, and their hidden tests must exercise those invariants deterministically.
- **Expert** tasks combine several independently testable components and require repository-level diagnosis; they must still have objective validation.

## Rust severity policy

- **Low** isolates one Rust mechanism behind a small API and a compact error or edge-case matrix.
- **Medium** requires a complete generic or ownership-aware component with several independent invariants but little cross-component coordination.
- **High** combines multiple modules or state transitions with ownership, error, parsing, concurrency, or architecture constraints that require iterative debugging.
- **Very high** combines several interacting correctness models such as concurrency plus eviction, or async cancellation plus bounded admission and shutdown. Its validator must use deterministic scheduling/control points and test failure recovery, not merely larger workloads.
