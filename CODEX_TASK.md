# Hase Benchmark Framework — Initial Development Task

## Objective

Build the first version of the `hase-bench` framework described in `AGENTS.md`.

This repository will eventually benchmark local LLMs running on the `hase` server through OpenCode and Pi.

The immediate development environment is:

```text
Windows 11
Visual Studio Community 2026
MSVC
CMake
Python
Git
```

Codex is developing the benchmark framework.

Codex itself is **not** the system being benchmarked.

The systems that will later be benchmarked are coding agents such as OpenCode and Pi connected to local models running on `hase`.

---

# Primary design requirement

Every programming task must support two modes.

## Manual

The user prepares a fresh task workspace, enters it, starts OpenCode manually and works interactively.

Example:

```powershell
python -m hasebench prepare cpp_001
cd <printed-directory>
opencode
```

Afterwards:

```powershell
python -m hasebench validate .
```

must perform authoritative validation.

## Autonomous

Later:

```powershell
python -m hasebench run cpp_001 --agent opencode --model <configuration>
```

must execute the same task automatically.

Do not make autonomous mode a prerequisite for manual mode.

---

# Initial implementation plan

Implement the framework incrementally.

The first working version should provide:

```text
task discovery
task metadata
workspace preparation
C++/CMake validation
visible tests
hidden tests
structured validation result
CLI
```

The initial CLI should support approximately:

```powershell
python -m hasebench list

python -m hasebench info cpp_001

python -m hasebench prepare cpp_001

python -m hasebench validate <workspace>
```

Do not implement all long-term features before these commands work.

---

# Initial C++ benchmark suite

Design the framework to support the following twenty-one C++ benchmark tasks.

The suite should deliberately cover substantially different capabilities.

Do not reduce the suite to twenty-one LeetCode-style functions.

Each task should be a small software project.

A task may require approximately several minutes to tens of minutes for a competent coding agent.

The harder tasks should require repository inspection, implementation, compilation and iterative debugging.

---

## CPP-001 — Expression Evaluator

Create a small arithmetic-expression library.

The starter project should expose an API that evaluates expressions such as:

```text
2 + 3 * 4
(2 + 3) * 4
-5 + 2
3.5 * 2
10 / 4
```

Requirements should include:

```text
operator precedence
parentheses
unary + and -
floating-point values
whitespace
syntax errors
division-by-zero handling
```

Do not require external parser libraries.

The intended implementation should require lexical analysis plus parsing.

Hidden tests should contain malformed expressions and difficult precedence combinations.

Category:

```text
parsing
algorithms
error handling
```

Difficulty:

`medium`

---

## CPP-002 — CSV Parser

Provide an incomplete CSV parser.

It must correctly handle:

```text
commas
empty fields
quoted fields
escaped quotes
embedded commas
embedded newlines
CRLF and LF
final line without newline
```

Expose a reasonably simple API returning rows and fields.

Do not require RFC-perfect CSV behavior beyond what TASK.md specifies.

Hidden tests must strongly exercise quoting and newline edge cases.

Category:

```text
parsing
state machine
edge cases
```

Difficulty:

`medium`

---

## CPP-003 — LRU Cache

Provide the API for a fixed-capacity generic LRU cache.

The implementation should provide approximately O(1):

```text
lookup
insert/update
eviction
```

Expected design may use:

```text
std::list
std::unordered_map
```

but TASK.md should specify behavior rather than implementation.

Test:

```text
capacity zero/one
replacement
access-order changes
update of existing keys
eviction order
move-only values if practical
```

Category:

```text
data structures
templates
API semantics
```

Difficulty:

`medium`

---

## CPP-004 — Thread Pool

Provide a partially implemented small thread pool.

Required functionality:

```text
fixed worker count
submit callable
return std::future
clean shutdown
exception propagation
multiple argument types
```

Hidden tests should check:

```text
many jobs
concurrent jobs
exceptions
shutdown behavior
no lost work
destructor draining of accepted work
```

Do not make timing assumptions unnecessarily strict.

Category:

```text
concurrency
RAII
templates
futures
```

Difficulty:

`hard`

---

## CPP-005 — SPSC Ring Buffer

Provide an incomplete single-producer/single-consumer bounded ring queue.

Require:

```text
non-blocking try_push
non-blocking try_pop
fixed capacity
correct wrap-around
support for movable values
```

Use atomics.

The task should require correct acquire/release memory ordering, not merely making all atomics sequentially consistent.

Hidden validation should include stress testing with producer and consumer threads and large operation counts.
It should also verify `Capacity == 1`, failed-pop output preservation, exact object lifetime through pop and destruction, and full publication of a non-trivial move-only payload. A narrow source-contract check should reject mutex locking and require an acquire/release order in the public header. Together these checks make the synchronization requirement objective without depending on arbitrary timing.

Do not require arbitrary MPMC behavior.

Category:

```text
atomics
memory ordering
concurrency
performance
```

Difficulty:

`very hard`

---

## CPP-006 — Binary Serialization

Create a small binary serialization/deserialization component.

Define a simple packet format containing fields such as:

```text
magic
version
flags
payload size
timestamp
payload
checksum
```

Require:

```text
fixed endianness
range validation
truncated-input detection
invalid-header detection
round trip
```

Specify one canonical byte order and exact field widths. Decoding must not read beyond the supplied span, must distinguish malformed headers, impossible lengths, truncation, and checksum failure, and must leave no partially decoded packet exposed on failure. Include payload sizes at zero, boundary, and configured-maximum values. Avoid unsafe pointer reinterpretation where possible.

Hidden tests should inject corrupted and truncated buffers.

Category:

```text
binary formats
byte manipulation
robustness
```

Difficulty:

`medium-hard`

---

## CPP-007 — Existing Bug: Lifetime / string_view

Provide a small realistic project with an intermittent or deterministic lifetime bug caused by inappropriate `std::string_view` usage.

The task description should say only that tests fail or sanitizable behavior is incorrect.

The defect must cross at least three files and two ownership boundaries: for example, a parser returns views into transient input which a cache and delayed formatter retain. The agent must diagnose and repair the lifetime problem while preserving the external API, copy/move behavior, and zero-copy behavior where ownership is genuinely external. Hidden tests should mix temporaries, reallocation, delayed formatting, and moved owning objects so a local "replace every string_view" patch is not automatically correct.

Category:

```text
debugging
lifetimes
modern C++
code comprehension
```

Difficulty:

`hard`

---

## CPP-008 — Existing Bug: Deadlock

Provide a component using multiple mutexes with a reproducible lock-ordering defect.

The API should represent something realistic, for example transferring values/resources between accounts or containers.

The task is to eliminate the deadlock without serializing the entire system through one global mutex. Include self-transfer, insufficient-resource, and exception/validation paths, and require the aggregate balance/resource invariant to hold. A throwing post-lock callback may overlap a successful transfer sharing one account; exception handling must not restore stale unlocked state over that successful change. Hidden tests should use barriers to deterministically exercise opposite-direction concurrent operations repeatedly, then verify both progress and the invariant. They should also gate the overlapping exception path and verify that an unrelated account pair continues to make progress. A solution must establish a consistent local locking strategy rather than relying on timeouts or retry sleeps.

Category:

```text
debugging
concurrency
mutexes
deadlocks
```

Difficulty:

`hard`

---

## CPP-009 — JSON-like Configuration Merge

Implemented as version 3. Do not require a full JSON parser.

Provide an existing lightweight configuration value type such as:

```cpp
variant<
    nullptr_t,
    bool,
    int64_t,
    double,
    string,
    vector<Value>,
    map<string, Value>
>
```

Ask the agent to implement recursive configuration merging.

Define precise semantics for:

```text
scalar replacement
object recursive merge
array replacement
type mismatch
missing keys
null
```

Hidden tests should deeply nest structures, distinguish an absent key from a present null, and verify that inputs are not aliased or accidentally mutated by the result.

The returned value must be an independent recursive copy: later mutation of an
array or object through the result must not alter either input, and later input
mutation must not alter the result. Arrays replace as a whole; they are not
concatenated or recursively merged. Null is an ordinary replacement value, not
a deletion marker. Integer and double are separate `variant` alternatives;
replacement does not perform numeric coercion.

Hidden validation also confirms that false, zero, an empty string, and an empty
array replace an existing value, and that a non-empty array replaces another
array wholesale rather than being merged element-by-element.

Category:

```text
variants
recursive algorithms
configuration handling
```

Difficulty:

`medium`

---

## CPP-010 — Graph Dependency Resolver

Implemented as version 1 in `tasks/cpp/cpp_010_dependency_resolver`. It uses a
fixed public result API with deterministic lexical ordering, diagnostic
precedence, and a canonical directed cycle path.

Required behavior:

```text
topological ordering
missing-dependency detection
cycle detection
stable/deterministic output
```

When a cycle exists, return a closed, useful cycle path if specified. Define lexical tie-breaking so output is deterministic across insertion order and unordered containers. Reject duplicate module definitions and make missing-dependency diagnostics identify both the dependent and the absent name.

Hidden tests should contain:

```text
disconnected components
diamonds
deep dependency chains
multiple valid orders
cycles
```

Category:

```text
graphs
algorithms
diagnostics
```

Difficulty:

`medium-hard`

---

## CPP-011 — Matrix / Least Squares

Implemented as version 2 in `tasks/cpp/cpp_011_least_squares`. The fixed,
row-major API makes its numerical and failure contract objectively testable:
stable QR least squares for full-column-rank overdetermined matrices, explicit
dimension/non-finite/rank errors, and scale-relative rank classification.
Visible validation establishes the basic API; hidden validation adds noisy and
higher-dimensional systems, residual integrity, close columns, badly scaled
independent columns, and error precedence. Version 2 adds an ordered large
later column whose remaining independent direction is below the required
original-column-relative rank threshold, preventing an incorrect threshold
based only on the diagonal of `R`. It intentionally rejects normal
equation shortcuts without prescribing a single QR implementation.

Provide a small matrix abstraction or fixed API without external libraries.

Ask the agent to implement a numerical least-squares solver for an overdetermined system.

Require a numerically reasonable QR-based solution rather than explicitly forming:

```text
(AᵀA)⁻¹Aᵀb
```

The contract must define behavior for rank-deficient or dimensionally invalid inputs (for example, a clear failure result), reject NaN/infinite inputs, and state a residual-based accuracy expectation rather than a single exact answer.

Validation should include:

```text
exact systems
noisy systems
different rectangular sizes
near-conditioning issues within reasonable tolerance
rank-deficient and badly scaled inputs
```

Use floating-point tolerance-based hidden validation against a trusted reference, including residual-quality checks that make a normal-equations implementation materially less reliable.

Category:

```text
numerical methods
linear algebra
algorithm design
```

Difficulty:

`very hard`

---

## CPP-012 — Quaternion rotation utilities

Implemented as version 1. Complete a compact C++20 right-handed rotation
library with normalization, axis-angle conversion, composition, inversion,
vector rotation, and conversion to and from row-major 3x3 matrices. The
contract fixes composition order, error values, near-zero behavior, and the
otherwise ambiguous quaternion sign. Matrix conversion accepts only finite
proper rotations with a stated orthogonality/determinant tolerance; it must
reject, rather than repair, reflections and skewed matrices.

Hidden validation checks vector/inverse/composition invariants rather than raw
quaternion equality, along with canonical sign behavior, scale independence,
matrix orthogonality and round trips, and all documented rejection paths.

Category:

```text
mathematics
geometry
numerical robustness
```

Difficulty:

`hard`

---

## CPP-013 — Streaming Log Statistics

Implemented as version 2. Build a C++20 single-line log ingester with a strict
four-field grammar, inclusive configured timestamp window, fixed severity
counters, malformed/outside precedence, a compensated mean of all accepted
latencies, and a fixed-capacity FIFO latency sample for nearest-rank p50/p95.
The implementation has bounded retained state and must not retain the complete
input stream. Hidden validation covers grammar boundaries, rolling percentiles,
configuration errors, time boundaries, adversarial numerical accumulation, and
long decimal conversion that must not discard later fractional digits.

Category:

```text
parsing
statistics
data processing
```

Difficulty:

`medium-hard`

---

## CPP-014 — HTTP Request Parser

Implemented as version 3 in `tasks/cpp/cpp_014_http_request_parser`.
The C++20 task implements a deliberately limited HTTP/1.1 request parser with
strict request-line and header grammar, case-insensitive header lookup that
preserves original spelling, exact `Content-Length` bodies, bounded headers and
bodies, arbitrary byte splits, pipelined requests, and a permanent error state.
It does not require TLS, chunked encoding, or a full RFC implementation.

Hidden validation replays a binary-body request at every split point, including
CRLF boundaries, and checks malformed input, duplicate and overflowing decimal
`Content-Length`, size limits, pipeline order, and preservation of already
completed requests after a later pipelined request fails. Version 3 adds the
overflow and completed-request checks under the existing written contract.

Category:

```text
protocols
state machines
incremental parsing
```

Difficulty:

`hard`

---

## CPP-015 — Generic Event Dispatcher

Create a type-safe event-dispatch component.

Clients should be able to subscribe callbacks for different event types and receive events.

Requirements may include:

```text
subscription token
unsubscribe
multiple subscribers
safe removal
move support
exception policy
```

Avoid requiring macros.

A strong solution may involve templates, type erasure, or `std::type_index`. Define dispatch semantics for callbacks that unsubscribe themselves or another subscriber, subscribe during dispatch, and throw exceptions. Subscription tokens must be safe after dispatcher moves and harmless when reset repeatedly. Hidden tests should make these re-entrant cases deterministic and verify that one event type cannot reach subscribers of another.

Category:

```text
templates
type erasure
architecture
API design
```

Difficulty:

`hard`

---

## CPP-016 — Resource Pool with RAII Handle

Provide a resource-pool API.

Resources are checked out through an RAII handle and automatically returned.

Test:

```text
move construction
move assignment
exception paths
pool exhaustion
resource reuse
destruction order
```

There should initially be at least one ownership bug or incomplete implementation. Require handles to remain valid through pool moves only when the stated API permits it, make moved-from handles inert, and define what happens when a handle is returned after a pool has begun destruction. Hidden tests should combine exception paths, nested handle scopes, move assignment between live handles, and resource reuse without accepting double return.

Category:

```text
RAII
ownership
move semantics
API design
```

Difficulty:

`hard`

---

## CPP-017 — Refactor Legacy Polymorphism

Provide an existing multi-file implementation that uses an awkward inheritance hierarchy and contains duplicated behavior.

Ask the agent to add a new feature that is deliberately inconvenient under the existing architecture.

The agent may refactor while preserving the public interface and existing behavior.

Validation focuses on behavior, not a predetermined architecture.

The project should be large enough that the model has to understand several files before editing. The requested feature should require a new behavioral axis that otherwise causes a combinatorial inheritance expansion, while old serialized/configured forms and public factories remain compatible. Hidden tests should instantiate legacy and new combinations, exercise error paths, and check behavior rather than a prescribed refactoring technique.

Category:

```text
maintenance
refactoring
architecture
repository navigation
```

Difficulty:

`very hard`

---

## CPP-018 — Image Processing Kernel

Represent an 8-bit grayscale image with width, height and stride.

Implement one or more operations such as:

```text
3x3 convolution
Sobel gradient
border handling
normalization/clamping
```

Correctly handle:

```text
non-contiguous row stride
small dimensions
borders
overflow/intermediate precision
```

Require aliasing policy, output-dimension rules, and deterministic border mode. Hidden tests compare against a trusted scalar reference implementation across odd strides, in-place/disallowed aliasing cases, tiny images, and large intermediate values.

A later version may include a performance target, but correctness comes first.

Category:

```text
image processing
memory layout
numeric code
```

Difficulty:

`medium-hard`

---

## CPP-019 — Concurrent Message Recorder

Implemented as version 2 in `tasks/cpp/cpp_019_message_recorder`. This C++20
multi-producer recorder accepts caller-supplied timestamped byte messages and
assigns gap-free sequence numbers to accepted submissions. One dedicated writer
passes them to a sink in sequence order. Capacity bounds only messages waiting
in the queue; the in-flight message does not occupy a slot, and a full queue
causes immediate rejection. Closing rejects new submissions and drains accepted
work before joining the writer. If the sink throws, writing stops and the failed
message plus queued accepted messages remain available through `undelivered()`;
successful persistence remains available through `snapshot()`.

The starter contains concurrency and shutdown defects. Visible and independent
hidden tests use controlled sink gates to check ordering, queue capacity,
concurrent submission, close/drain behavior, failure recovery, and clean thread
teardown without relying on random scheduling. Version 2 additionally compares
the actual sink arguments with persisted records and checks failure reporting,
submission rejection, and recovery before an explicit close.

Category:

```text
concurrency
systems programming
debugging
architecture
```

Difficulty:

`very hard`

---

## CPP-020 — Integrated Mini Service

Implemented as version 1 in `tasks/cpp/cpp_020_mini_service`. This expert
C++20 task is a five-file service with configuration parsing, incremental
legacy/tagged record parsing, transactional queue admission, a dedicated sink
worker, and per-source alert statistics. The starter compiles but contains
interacting ownership, format, accounting, admission, and shutdown/failure
defects. The requested source-aware feature crosses every component.

The public contract fixes configuration and record grammars, limits, alert
thresholds, queue capacity, all-or-nothing `ingest` behavior, retry after
rejection, sink-failure recovery, and draining close. The agent-visible
`TASK.md` describes behavior without identifying defect locations. Independent
hidden targets report component and integration failures separately. They
exercise source ownership after input mutation, every record split point,
malformed records, per-source statistics, partial-line retry after queue
rejection, batch rollback, sink failure, repeated close, and legacy input.
The author solution passes both targets and visible validation on MSVC.

Category:

```text
integration
debugging
architecture
repository comprehension
multi-step reasoning
```

Difficulty:

`expert`

---

## CPP-021 — SIMD PCA / Covariance Kernel

Implemented as version 2 in `tasks/cpp/cpp_021_simd_pca`. The C++20 project
uses a bounded eight-feature API with explicit stride, error results, backend
request/reporting, population covariance, all eigenvalues, leading-component
axes, and projection/reconstruction. Its independent hidden target compares
against scalar moments and checks eigenpair residuals, ordering, orthogonality,
sign, dimensions, non-finite values, padded stride, badly scaled data,
degenerate eigenspaces, and forced scalar/AVX2 dispatch. AVX2 is compiled only
for its separate covariance source file and selected at runtime.
Version 2 adds independent eight-feature dispatch and uniformly tiny,
representable rank-one covariance checks. The latter detects an absolute
Jacobi stopping threshold that loses a valid principal component; the
agent-visible task contract is unchanged.

Provide a small C++20 numerical component that accepts row-major floating-point
observations with an explicit row stride and computes:

```text
per-feature mean
centered covariance matrix
the leading K principal components
projection of an observation onto those components
```

Keep the feature dimension deliberately bounded so a deterministic symmetric
Jacobi eigensolver is practical without external numerical libraries. Define
the behavior for zero observations, invalid stride/dimensions, non-finite
values, zero-variance features, repeated eigenvalues, component ordering, and
eigenvector sign. Validation must compare covariance, eigenvalue ordering, and
projection/reconstruction quality to a trusted scalar reference within stated
tolerances; it must not require one arbitrary basis within a degenerate
eigenspace.

The project must include both a scalar implementation and an optional AVX2
acceleration path for the center/covariance accumulation. Runtime CPU-feature
detection selects AVX2 only when available and otherwise selects scalar. The
public API must report the selected backend so validation can exercise and
compare both paths. The scalar path remains required and correct on every
supported machine; compilation must not globally require AVX2. SIMD and scalar
results are compared with numerical tolerances, not bitwise equality.

Hidden tests should include:

```text
non-contiguous row stride
small and rectangular datasets
badly scaled features
zero-variance and repeated-eigenvalue data
non-finite input rejection
forced scalar and supported AVX2 dispatch
projection/reconstruction residual checks
```

Do not impose a raw throughput threshold in the first version. Record optional
performance telemetry later, after correctness and portable dispatch are
stable.

Category:

```text
numerical methods
linear algebra
SIMD
CPU feature dispatch
memory layout
```

Difficulty:

`expert`

---

# Difficulty balance

The initial suite should approximately span:

```text
medium
medium-hard
hard
very hard
expert
```

Do not make all tasks equally difficult.

A smaller model should still pass some tasks.

A strong model should not trivially pass everything.

The purpose is to create useful score separation.

---

# Task construction principles

Every task must have an objective validator.

Avoid relying on subjective judgment for correctness.

Prefer behavioral requirements over implementation requirements unless the implementation technique itself is what is being tested.

For example, CPP-005 intentionally tests atomics/memory ordering and may therefore impose stronger implementation constraints.

Keep hidden validation independent of agent-written tests.

Do not expose expected outputs so completely that the model can hard-code answers.

Use deterministic tests where possible.

Concurrency stress tests should minimize random flakiness.

Use fixed random seeds when random test generation is useful.

---

# Agent-visible instructions

Every prepared workspace must contain:

```text
TASK.md
```

TASK.md should describe only what a normal developer should know.

It may contain:

```text
problem description
requirements
API constraints
build instructions
available tests
language level
prohibited changes
```

It must not expose:

```text
hidden tests
reference implementation
benchmark scoring internals
expected patch
```

---

# Manual benchmark experience

The framework must make the following workflow pleasant.

The user chooses:

```text
cpp_005
```

Then:

```powershell
python -m hasebench prepare cpp_005
```

The command prints something similar to:

```text
Workspace:
D:\Projects\hase-bench\work\20260910-214200_cpp_005_manual
```

The user enters that directory and starts:

```powershell
opencode
```

The first instruction may simply be:

```text
Read TASK.md and solve the task.
You may inspect the repository, build it, run tests and modify the implementation.
```

The user may then interact with OpenCode normally.

Afterwards:

```powershell
python -m hasebench validate .
```

should print a concise result.

For example:

```text
Task:       cpp_005
Build:      PASS
Visible:    14/14
Hidden:     27/30
Result:     FAIL
```

Detailed diagnostics should be available without overwhelming the default output.

---

# Automatic benchmark experience

Later support approximately:

```powershell
python -m hasebench run cpp_005 `
    --agent opencode `
    --model qwen38-27b-q6
```

The autonomous agent should receive essentially the same initial instruction used in manual mode.

The benchmark must then validate the result independently.

Manual and autonomous benchmark scores should not be mixed without recording their mode.

---

# Result comparison

Eventually support something approximately like:

```powershell
python -m hasebench compare `
    --model qwen38-27b-q4 `
    --model qwen38-27b-q6
```

Possible report:

```text
                     Q4       Q6
CPP-001             PASS     PASS
CPP-002             PASS     PASS
CPP-003             FAIL     PASS
...
CPP-020             FAIL     FAIL
CPP-021             FAIL     FAIL

Solved              11/21    15/21
Build failures       3        1
Hidden failures      6        4
```

Do not implement this report before basic execution and validation are stable.

---

# Extension plan

The architecture must make new tasks mostly data/project additions rather than modifications to the benchmark engine.

Adding another C++ task should ideally require:

```text
new task directory
metadata
starter repository
validator
```

and no changes to the generic framework.

---

# Rust extension

After C++ is established, add:

```text
tasks/rust/
```

and a `RustCargoValidator`.

Use a four-level Rust planning severity scale: `low`, `medium`, `high`, and
`very high`. These map to task metadata `easy`, `medium`, `hard`, and
`very hard` when the task is implemented. Severity reflects the number and
interaction of correctness invariants, not source size or test volume. The
planned suite must contain accessible work as well as multiple High and
Very-high tasks capable of separating stronger models.

`rust_001` implements the first ownership/lifetime repair task as an owned
configuration snapshot. Its starter intentionally exposes lifetime-parameterized
entries borrowed from the parse input. The required Rust 2024 API owns parsed
keys and values, accepts owned updates, and produces independent clones. Visible
tests cover parsing and ordinary updates; the external Cargo validator enforces
source destruction, owned arguments, clone independence, ordering, and edge
cases. Version 2 also compiles explicit checks for every required public trait
and verifies that case-insensitive key matching remains ASCII-only; the
agent-visible contract is unchanged from version 1.

`rust_002` implements the generic-iterator task as a lazy sorted merge join.
The adaptor accepts differently typed input iterators plus a mutable comparator
and yields left-only, right-only, or paired values. It must support move-only
items, pair duplicates one-for-one, buffer at most one item per side, provide
the specified lower/upper `size_hint`, and conditionally implement
`FusedIterator`. Independent tests cover each of those generic and consumption
invariants.

`rust_003` implements the focused error-propagation task as a sequential
counter-update pipeline. Text operations are parsed before existing counters
are looked up and checked signed additions are applied. Failures preserve a
zero-based operation index and typed parse, lookup, or update causes through
`From` and a two-level `Error::source` chain. Earlier successful operations
remain applied, while the failing and later operations make no changes.
Independent tests cover the explicit precedence matrix, trimming and key
semantics, both integer limits, partial progress, empty input, and public trait
contracts. It remains intentionally Low severity without rollback or
concurrency.

`rust_004` implements the bounded channel worker as a generic Rust 2024
component. Multi-producer non-blocking admission assigns gap-free sequence
numbers only to accepted move-only jobs. One dedicated thread borrows jobs for
handling, publishes successful results in order, drains on repeated close and
drop, and on a typed handler failure recovers the failed job followed by every
queued accepted job. Capacity counts only waiting jobs, not the in-flight job.
Visible and independent hidden tests use channel gates rather than sleeps to
prove backpressure, sequence continuity, ordered concurrent admission, drain,
failure stop/recovery, and ownership preservation.
The failure state is permanent: a later `close()` must retain both
`WorkerStatus::Failed` and the failed-submission classification.

`rust_005` implements a dependency-free incremental binary frame parser. Its
exact wire format combines magic, version, kind, a big-endian payload length,
arbitrary payload bytes, and an FNV-1a integrity field. The parser accepts any
chunking, publishes multiple frames in order, enforces its payload bound as
soon as the header is complete, retains frames completed before a later error,
and makes malformed/truncated input permanently observable. Independent tests
replay every byte split and cover error precedence, exact truncation state,
checksum diagnostics, ownership draining, and post-finish behavior.

`rust_006` implements a multi-file trait-driven storage refactor. The legacy
memory-backed document service gains object-safe `Storage`, `Transaction`, and
middleware boundaries while retaining its concrete compatibility path and
non-generic public `DocumentService` type; the constructor alone is generic
over storage implementations.
Whole-batch key validation precedes transaction creation; middleware runs in
defined registration/reverse order; all post-begin failures roll back exactly
once; and rollback failure retains the original typed error. Independent
hidden backends exercise begin, operation, commit, and rollback failures,
transaction isolation/closure, caller-owned results, and middleware ordering
without downcasting or implementation-specific hooks.

`rust_007` implements a generic concurrent single-flight cache using only the
standard library. Its written state model separates absent, loading, and ready
keys; loaders execute outside cache locks, waiters share one typed flight
result, and same-thread same-key recursion fails instead of deadlocking.
Deterministic hashing assigns exact per-shard quotas, injected ticks define
publication-based TTL, ready hits update LRU-style recency, and in-flight work
does not consume capacity. Panic/error cleanup and idempotent close both wake
waiters, while late loaders cannot repopulate a closed cache. Hidden channel
and barrier orchestration checks progress, cleanup, eviction, expiry, and
shutdown without relying on randomized stress.

`rust_008` implements a cancellation-safe async service on pinned Tokio
1.47.1. Synchronous non-blocking admission counts every queued or active
request, assigns gap-free accepted sequence numbers, and returns caller-owned
requests on rejection. Accepted work is serialized per stream while unrelated
streams progress independently. Dropping a handle cancels queued or active
work, and one injected-clock deadline spans both phases without leaking the
capacity permit. The first transport failure is permanent. Graceful close
rejects new work, drains accepted requests, calls transport close exactly once,
and continues even if its initiating caller is cancelled. Independent tests
use controlled transport futures and an injected manual clock to prove the
ordering, cleanup, progress, deadline, failure, and shutdown invariants without
wall-clock sleeps.

For High tasks, the design review must identify at least two interacting state
or ownership boundaries and the hidden validator must test their failure paths.
For Very-high tasks, write the state/concurrency model before the starter and
use deterministic orchestration to prove progress, cleanup, and recovery. A
larger random stress loop alone does not justify Very-high severity.

Reserve `rust_rsync` as a single Very-high, long-horizon cross-platform rsync-clone
task. The agent must build a complete functional Rust implementation, runnable
on both Windows and Linux, rather than a local-only file copier. At task
authoring time, pin an upstream rsync release and its official `rsync(1)` and
`rsyncd.conf(5)` manuals as the compatibility baseline. The contract must
cover local, remote-shell, and daemon modes; interoperability with the pinned
upstream implementation; the delta-transfer protocol; option parsing and
filter rules; traversal, deletion, partial/resumed transfers, checksums,
preservation of supported metadata, diagnostics, and exit codes. Specify
Windows drive/UNC paths, case behavior, reparse points, and filesystem
capability differences alongside Linux permissions, links, and metadata.
Unavailable OS features may have explicit capability-based behavior, but the
task must not quietly omit an entire transfer mode or supported feature.

This remains one benchmark project. Its starter should provide a modular CLI,
transport, protocol, filesystem, and transfer skeleton with visible integration
tests. The independent validator should test both operating systems and use
the pinned upstream rsync as a differential peer for local, remote-shell, and
daemon scenarios, including interrupted transfers and adversarial paths. Give
the task a substantially larger agent/build/test time budget than the compact
Rust tasks. Do not claim completion from Linux-only validation or from a
partial feature subset.
Establish the Rust validator and workflow with smaller tasks before building
this capstone; its reserved ID does not imply it must be implemented first.
Follow the design, implementation, review, test, and progress checkpoints in
`RUST_RSYNC_PLAN.md` when authoring and running this task.

The same:

```text
prepare
manual solve
validate
autonomous run
```

workflow must work.

---

# General-task extension

Add:

```text
tasks/general/
```

These need not contain compilable projects.

Support validators such as:

```text
ExactAnswerValidator
NumericToleranceValidator
JsonSchemaValidator
PropertyValidator
```

Possible benchmark categories:

```text
mathematical reasoning
algorithm analysis
code comprehension
log diagnosis
structured extraction
technical decision making
instruction following
```

General tasks should still use objective scoring whenever practical.

---

# Visual/game extension

Later add:

```text
tasks/visual/
```

The first visual task may be a very small C++ game such as:

```text
Pong
Breakout
simple particle simulation
```

The task could ask the coding agent to repair or add behavior.

Do not score it solely from whether a window appears.

Prefer deterministic game-state validation.

For example:

```text
ball collision equations
score changes
paddle constraints
fixed-step simulation
game reset
```

A screenshot/render comparison may be added as a secondary validation layer.

This extension must not influence the first benchmark implementation.

---

# What to implement now

Start with the repository infrastructure and only a small subset of the twenty-one tasks.

Recommended implementation order:

```text
CPP-001 Expression Evaluator
CPP-003 LRU Cache
CPP-005 SPSC Ring Buffer
```

These give three substantially different test types:

```text
parser
generic data structure
concurrent systems programming
```

Do not implement all twenty-one before validating the framework design.

Once these three work manually, add autonomous OpenCode execution.

Then grow the C++ suite progressively.

---

# Required first deliverable

Produce a repository in which this sequence works:

```powershell
python -m hasebench list

python -m hasebench prepare cpp_001

cd <workspace>

# user runs OpenCode manually and solves TASK.md

python -m hasebench validate .
```

The resulting workspace must remain available after validation.

The canonical benchmark task under `tasks/` must remain unchanged.

The validator must use at least some tests that are unavailable to the coding agent.

Once this pipeline works for CPP-001, implement CPP-003 and CPP-005 using the same framework.

Do not implement autonomous OpenCode control until the manual workflow is working cleanly.

---

# Development expectations for Codex

Read `AGENTS.md` before implementing.

Inspect the repository before making architectural assumptions.

Keep the implementation Windows-native and MSVC-compatible.

Use clear Python with type annotations.

Prefer the standard library where practical.

Avoid introducing large framework dependencies without a concrete need.

Use tests for the benchmark framework itself.

Do not modify benchmark requirements merely to make implementation easier.

Maintain a short `README.md` explaining how the user performs the current supported workflow.

Make incremental commits or at minimum keep changes logically separable.

If the implementation reveals that part of this specification is unnecessarily complex, simplify the implementation while preserving:

```text
clean canonical tasks
isolated run workspaces
manual OpenCode workflow
objective independent validation
future autonomous mode
future C++/Rust/general/visual extensibility
```
