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

## CPP-012 — Geometry / Quaternion

Provide incomplete quaternion/3D rotation utilities.

Required behavior may include:

```text
normalization
multiplication
inverse/conjugate
rotate vector
axis-angle conversion
rotation-matrix conversion
```

Specify zero/near-zero normalization behavior, the quaternion sign ambiguity, and how invalid rotation matrices are rejected or normalized. Hidden tests should verify mathematical invariants rather than only example values.

Examples:

```text
norm
inverse identity
rotation composition
matrix orthogonality
round trip
```

Category:

```text
mathematics
geometry
numerical robustness
```

Difficulty:

`hard`

---

## CPP-013 — Log Parser and Statistics

Provide several sample structured/unstructured log lines.

Implement a streaming parser that extracts fields and produces statistics such as:

```text
message counts
latency mean
percentiles
errors by category
time-window aggregation
```

Require a documented bounded-memory streaming strategy for aggregate metrics; if exact percentiles require retained samples, specify an explicit bounded window instead. Define malformed-record accounting, timestamp inclusivity at window boundaries, and merge/order behavior for equal timestamps. Tests should exercise malformed records, boundary timestamps, large streams, and numeric stability for latency aggregation.

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

Implement a deliberately limited HTTP/1.1 request parser.

Scope should be clearly specified.

Support approximately:

```text
request line
headers
case-insensitive header names
Content-Length body
incremental input
```

Do not require:

```text
TLS
chunked encoding
full RFC implementation
```

The important difficulty is incremental parsing: data may arrive split at arbitrary byte boundaries, including within CRLF, header names, and the body. Specify a bounded header/body limit, duplicate `Content-Length` policy, case-insensitive lookup while preserving values, pipelined remainder handling, and a clear error state that cannot be resumed accidentally after malformed input. Hidden tests should systematically replay the same requests at every possible split point.

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

Create a miniature version of a realistic producer/writer architecture.

A producer submits timestamped byte messages.

A dedicated consumer/writer persists them into a mock or in-memory sink.

Requirements should include:

```text
bounded queue
clean shutdown
draining
preserved message ordering
no corruption
backpressure/drop policy as specified
```

The starter implementation should contain at least one concurrency or shutdown defect. Specify a deterministic backpressure/drop policy, a monotonic sequence contract, and how writer failure is reported without silently losing accepted messages. Hidden tests should use controlled producer/writer gates for bursts, full queues, writer failure, and shutdown while data remains queued; they must verify exactly-once persistence of all accepted messages and clean thread teardown.

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

This is the most difficult initial C++ task.

Provide a small multi-file service containing several components, for example:

```text
configuration
message parser
worker queue
statistics
serialization
```

The project should initially compile but contain several interacting logical defects across at least four components, including one ownership or lifetime defect and one shutdown/error-propagation defect. A requested feature should cross parser, configuration, queue, and statistics boundaries rather than be solvable in one file.

TASK.md should describe observed incorrect behavior and requested feature changes without pointing directly to the defects.

The agent should need to:

```text
inspect several files
understand interactions
run tests
diagnose multiple issues
modify multiple components
re-run tests
```

Use visible tests for basic behavior and hidden tests for integration behavior, failure rollback, repeated lifecycle operations, and compatibility with a small legacy input corpus. The validator should report independent component failures where practical so results distinguish superficial fixes from real repository-level understanding.

This task should intentionally distinguish stronger local coding models from weaker ones.

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

Initial Rust benchmark ideas include:

```text
ownership/lifetime repair
generic iterator implementation
channel-based worker
concurrent cache
parser
error propagation
trait-based architecture
async service component
```

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
