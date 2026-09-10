# Hase Agent Benchmark

## Purpose

This repository implements a reproducible benchmark for evaluating local LLMs used through coding agents such as **OpenCode** and **Pi**.

The LLMs themselves run remotely on the Linux server `hase`.

The benchmark framework runs on a Windows 11 development workstation.

The benchmark should answer practical questions such as:

* How capable is model A compared with model B for real software-development work?
* How much quality is lost when moving from Q8 to Q6, Q5, or Q4 quantization?
* Does the same model perform differently through OpenCode and Pi?
* Does llama.cpp versus vLLM materially affect agent behavior?
* Can the model diagnose existing code rather than merely generate isolated functions?
* Can it navigate a multi-file project?
* Can it understand APIs and invariants?
* Can it fix compilation errors?
* Can it debug failing tests?
* Can it reason about modern C++?
* Can it work with concurrency, templates, parsing, algorithms, networking, numerics, and architecture?
* How much human intervention is required?

This is primarily an **agent benchmark**, not a raw prompt-completion benchmark.

---

# Roles

There are three conceptually separate components.

## 1. Benchmark framework

Developed in this repository using Codex.

It prepares benchmark tasks, launches agents when requested, builds projects, executes validation tests, records results, and produces reports.

## 2. Coding agent under test

Initially:

* OpenCode

Later:

* Pi
* possibly other agents

The coding agent receives a software-development task and modifies a clean working copy of the corresponding project.

## 3. Model under test

The coding agent communicates with a model running on `hase`.

Models may run through:

* llama.cpp / llama-server
* vLLM

Different models and quantizations will be evaluated.

The benchmark framework must not assume a specific model.

---

# Environment

## Benchmark workstation

Operating system:

* Windows 11

Development environment:

* Visual Studio Community 2026
* current MSVC toolchain
* CMake
* PowerShell
* Python
* Git

There is no requirement to restrict benchmark tasks to C++17.

Modern C++ features supported by the installed MSVC version may be used where they improve a task.

However, tasks should explicitly state their required C++ language level.

Prefer C++20 or C++23 for new benchmark projects unless a task specifically targets older language behavior.

Later the workstation will also contain:

* Rust
* Cargo

---

# Hase inference server

Hostname:

`hase`

Relevant configuration:

* Ubuntu Server
* Intel Core Ultra 5 225
* 128 GB system RAM
* Intel Arc Pro B70 Creator
* 32 GB GPU memory
* Intel XPU software stack
* llama.cpp
* vLLM
* multiple local Qwen-family models and quantizations

Models are served remotely over the LAN.

The exact:

* server address
* port
* backend
* model identifier
* context size
* temperature
* seed
* generation settings

must be configuration data.

Do not hard-code them into the benchmark implementation.

---

# Core benchmark concept

Each benchmark task is a small but realistic software project.

A benchmark run operates on a **copy** of that project.

The canonical task is never directly modified.

Conceptually:

```text
canonical task
      |
      | copy
      v
run workspace
      |
      v
OpenCode / Pi
      |
      | talks to model on hase
      v
agent modifies project
      |
      v
benchmark validator
      |
      +-- configure
      +-- build
      +-- visible tests
      +-- hidden tests
      +-- optional static checks
      |
      v
result
```

---

# Manual and automatic modes

The same benchmark task must support both.

## Manual mode

The user creates a working copy and starts OpenCode interactively.

Example conceptual workflow:

```powershell
python tools\bench.py prepare cpp_001
cd work\<generated-run-directory>
opencode
```

The user may then communicate interactively with the model.

For example:

```text
Read TASK.md and solve the task.
Build the project and run the available tests.
```

The user may continue the discussion, ask questions, or request corrections.

This run should later be recorded as:

`manual`

or:

`assisted`

rather than an autonomous benchmark.

The framework should make it easy to validate the workspace afterwards:

```powershell
python tools\bench.py validate .
```

## Automatic mode

The framework creates the same clean working copy and launches OpenCode or Pi non-interactively.

Conceptually:

```powershell
python tools\bench.py run cpp_001 --agent opencode --configuration qwen-q6
```

The framework then:

1. creates a clean workspace;
2. starts the selected agent;
3. provides the task instruction;
4. waits for the agent to terminate;
5. independently builds the result;
6. independently executes authoritative validation;
7. saves logs and metadata;
8. reports the result.

Manual and automatic runs must use the same canonical task definitions.

---

# Repository layout

Use a structure approximately like:

```text
hase-bench/
│
├── AGENTS.md
├── README.md
│
├── pyproject.toml
│
├── config/
│   ├── agents/
│   ├── models/
│   └── machines/
│
├── tasks/
│   ├── cpp/
│   ├── rust/
│   ├── general/
│   └── visual/
│
├── tools/
│
├── src/
│   └── hasebench/
│
├── work/
│
├── results/
│
└── docs/
```

The exact Python package layout may evolve, but responsibilities should remain separated.

---

# Task layout

A programming benchmark task should conceptually contain:

```text
tasks/cpp/cpp_001_example/
│
├── task.yaml
├── TASK.md
│
├── starter/
│   ├── CMakeLists.txt
│   ├── include/
│   ├── src/
│   └── tests/
│
├── validator/
│   ├── CMakeLists.txt
│   ├── hidden_tests/
│   └── validate.py
│
└── reference/
```

`starter/` is copied into the agent workspace.

`validator/` stays outside the workspace.

`reference/` may contain expected implementations, design notes, or benchmark-author-only information.

The agent must not receive `validator/` or `reference/`.

---

# Visible versus hidden tests

Tasks may contain two kinds of tests.

## Visible tests

These are part of the starter project.

The coding agent may:

* inspect them;
* execute them;
* use them for debugging.

They represent the normal developer experience.

## Hidden tests

These are owned by the benchmark framework.

They are not copied into the workspace.

They verify:

* edge cases;
* invariants;
* API preservation;
* behavior not directly demonstrated by visible tests;
* robustness against overfitting to visible tests.

A solution passing visible tests but failing hidden tests is not considered fully correct.

---

# Run workspaces

Every benchmark attempt gets a separate directory.

For example:

```text
work/
└── 20260910-213412_cpp_007_opencode_qwen-q6/
```

A workspace should contain:

```text
TASK.md
source tree
visible tests
agent-generated files
build directory
agent log
run metadata
```

Do not reuse a previous workspace for an autonomous benchmark.

Do not automatically delete workspaces during the initial development stage.

They are valuable debugging artifacts.

Later implement explicit cleanup commands.

Examples:

```powershell
python tools\bench.py clean --successful
python tools\bench.py clean --older-than 30d
```

Never silently delete failed runs.

---

# Git

The benchmark repository itself must be a Git repository.

Canonical tasks belong to Git.

`work/` and large generated result artifacts should normally be excluded from Git.

Results intended for long-term comparison may be stored as compact JSON/JSONL files under version control or exported separately.

Use `.gitignore` appropriately.

A workspace may optionally be initialized as its own temporary Git repository so that agent changes can be inspected with:

```text
git diff
```

This would be useful but is not required for the first milestone.

---

# Task metadata

Each task should eventually expose machine-readable metadata.

For example:

```yaml
id: cpp_007
title: Concurrent bounded queue repair
language: cpp
standard: c++20

category:
  - debugging
  - concurrency

difficulty: hard

timeout:
  agent_seconds: 900
  build_seconds: 120
  test_seconds: 30
```

Do not over-engineer the schema initially.

Start with fields needed by the first few tasks.

---

# Result model

Each benchmark run should eventually record:

```text
run id
timestamp

task
task version

agent
agent version

model
backend
quantization

generation configuration

manual / assisted / autonomous

agent exit status
agent wall time

build result
build duration

visible test result
hidden test result

number of tests passed
number of tests failed

overall result

workspace path

agent output/log path
```

Later extend this with:

```text
input tokens
output tokens
TTFT
tokens/s
context usage
tool calls
number of build attempts
number of test attempts
number of modified files
diff size
```

Quality/correctness and inference performance must remain distinct metrics.

---

# Failure classification

Do not collapse all failures into one category.

At minimum distinguish:

```text
AGENT_FAILURE

TIMEOUT

NO_SOLUTION

BUILD_CONFIGURATION_FAILURE

COMPILATION_FAILURE

LINK_FAILURE

VISIBLE_TEST_FAILURE

HIDDEN_TEST_FAILURE

RUNTIME_CRASH

RUNTIME_TIMEOUT

API_CONTRACT_FAILURE

SUCCESS
```

This is important for comparing model behavior.

---

# Agent adapters

The framework must not depend directly on OpenCode.

Define a narrow abstraction around agent execution.

Conceptually:

```python
class AgentRunner:
    def run(request: AgentRunRequest) -> AgentRunResult:
        ...
```

Implement:

```text
OpenCodeAgentRunner
```

first.

Later add:

```text
PiAgentRunner
```

without modifying task definitions.

---

# Validation adapters

Likewise separate task validation from agent execution.

Examples:

```text
CppCMakeValidator
RustCargoValidator
ExactAnswerValidator
JsonValidator
VisualValidator
```

A coding task should not need to know how OpenCode was launched.

---

# C++ validation

For C++ tasks:

* use CMake;
* use MSVC;
* prefer out-of-source builds;
* execute visible tests when appropriate;
* execute hidden benchmark tests independently;
* capture compiler stdout/stderr;
* capture test stdout/stderr;
* enforce process timeouts;
* preserve artifacts when validation fails.

Avoid shell-string construction from model-generated input.

Use subprocess argument arrays.

---

# Safety

Agent-generated programs are untrusted code.

During early development, tasks will be benchmark-authored and relatively controlled, but process boundaries and timeouts must still be used.

Do not allow generated code to control:

* compiler executable path;
* validation command line;
* benchmark directories;
* result paths.

Later investigate stronger execution isolation.

Do not make sandboxing a prerequisite for the first working version.

---

# C++ benchmark philosophy

Avoid a suite consisting mostly of algorithm puzzles.

The C++ suite should measure different software-engineering capabilities.

Include:

* new implementation;
* debugging;
* maintenance;
* API understanding;
* multi-file navigation;
* templates;
* concepts;
* ownership;
* RAII;
* error handling;
* parsing;
* concurrency;
* atomics;
* numerical programming;
* data structures;
* serialization;
* performance reasoning;
* modern C++;
* architecture/refactoring.

Some tasks should be easy.

Some should deliberately require several reasoning/build/debug cycles.

The difficult tasks should be capable of distinguishing a strong 20B–30B local coding model from a weaker model.

---

# Rust benchmark philosophy

Rust tasks will be added after the C++ framework is stable.

Use exactly the same benchmark concepts:

```text
starter repository
visible cargo tests
hidden cargo tests
manual mode
automatic agent mode
result capture
```

Rust-specific tasks should exercise:

* ownership
* borrowing
* lifetimes
* iterators
* enums
* error handling
* traits
* generics
* concurrency
* channels
* async
* serde-style data handling where dependencies are appropriate

Do not add Rust support until the generic task/agent/validator interfaces exist.

---

# General reasoning benchmarks

Not every benchmark should involve modifying source code.

Later support task categories such as:

```text
general/
```

Examples:

* mathematical reasoning;
* code comprehension;
* structured extraction;
* instruction following;
* debugging diagnosis without editing;
* architecture decisions;
* log analysis;
* algorithm analysis.

These may be scored using:

* exact answers;
* numerical tolerances;
* JSON schema validation;
* reference-property checking;
* deterministic scripts.

Do not use another LLM as the primary judge when an objective validator is possible.

---

# Visual benchmark tasks

Visual tasks are a later extension.

One possible category is a small visual application or game.

For example:

```text
visual/
    breakout
    pong
    particle simulation
```

The agent might be asked to implement or repair rendering/game behavior.

Validation may combine:

* compilation;
* deterministic simulation;
* game-state assertions;
* generated screenshots;
* image comparison;
* pixel or perceptual thresholds.

Avoid making subjective screenshot judgment the only criterion.

Where possible, test internal game state deterministically.

Visual tasks are explicitly out of scope for the first implementation.

---

# Benchmark versioning

Task behavior must not silently change.

Once benchmark results are being compared, modifying:

```text
TASK.md
starter code
visible tests
hidden tests
validator
```

changes the benchmark.

Tasks should therefore eventually have a version.

For example:

```text
cpp_007 version 1
```

If hidden tests materially change, increment the version.

---

# Development stages

## Stage 1 — Framework skeleton

Implement:

```text
repository structure
task discovery
workspace creation
one C++ task
manual preparation
manual validation
```

No autonomous agent launch is required yet.

Target commands:

```powershell
python -m hasebench list

python -m hasebench prepare cpp_001

python -m hasebench validate <workspace>
```

## Stage 2 — Autonomous OpenCode

Add:

```text
OpenCodeAgentRunner
run metadata
agent logs
automatic validation
JSON results
```

Target:

```powershell
python -m hasebench run cpp_001 --agent opencode --model qwen-test
```

## Stage 3 — Initial C++ suite

Implement the 20 C++ benchmark projects defined in `CODEX_TASK.md`.

Do not necessarily implement all twenty before testing the framework.

Introduce them incrementally.

## Stage 4 — Reporting

Add:

```text
task summary
per-model score
failure classification
comparison table
CSV/JSON export
```

Do not start with a web dashboard.

A good CLI report is sufficient.

## Stage 5 — Pi

Add Pi through a second AgentRunner implementation.

Do not change benchmark projects to accommodate Pi.

## Stage 6 — Rust

Implement RustCargoValidator and add initial Rust tasks.

## Stage 7 — General benchmarks

Add deterministic non-programming reasoning tasks.

## Stage 8 — Performance telemetry

Capture:

```text
tokens
TTFT
generation throughput
context
agent turns
tool calls
```

where the relevant backend/agent exposes them reliably.

## Stage 9 — Visual tasks

Add one small deterministic visual/game benchmark.

---

# Important implementation rule

Do not attempt to implement the final architecture in one step.

Codex should work incrementally and keep the repository runnable.

Prefer:

```text
small implementation
test
commit
extend
```

over a large speculative framework.

---

# First concrete milestone

The first milestone is complete when the following works:

```powershell
python -m hasebench list
```

shows the available task.

Then:

```powershell
python -m hasebench prepare cpp_001
```

creates a clean workspace and prints its path.

The user can:

```powershell
cd <workspace>
opencode
```

and manually solve the task.

Finally:

```powershell
python -m hasebench validate <workspace>
```

independently builds and validates it.

Only after this manual pipeline is reliable should autonomous OpenCode execution be implemented.

Create and support `progress.md` file weher you will put all finsihed stages and current one you work on. 

When user ask your to continue, check what stage is active/pending in `progress.md`
