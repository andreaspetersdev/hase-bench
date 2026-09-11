# Hase Bench progress

## Completed

- Repository layout, task discovery, workspace preparation, and framework unit tests.
- CPP-001 Expression Evaluator, CPP-003 Generic LRU Cache, and CPP-005 SPSC Ring Buffer canonical tasks, each with visible and separate hidden tests.
- Stage 1 — Framework skeleton verified end-to-end with real MSVC for CPP-001, CPP-003, and CPP-005.
- Manual workflow: `validate --all` discovers only metadata-marked direct children of `work/`, supports `--task`, continues after corrupt workspaces, and prints per-run plus aggregate outcomes.
- Manual preparation supports `--label` for an optional safe model/run suffix; new workspace names use the concise `_man` marker.
- Validation reports show each task's declared complexity for both individual and batch runs, using compact labels (`E`, `M`, `H`, `VH`).
- Benchmark-specification review of CPP-001, CPP-003, and CPP-005 based on real manual model runs; all three task versions are now version 2.
- Stage 2 — Autonomous OpenCode execution: `run` creates a fresh `_aut_opencode` workspace, invokes OpenCode non-interactively through a runner adapter, preserves JSON agent output plus structured run metadata, and independently reuses the manual CMake validator.
- Autonomous runs support the same safe `--label` suffix as manual preparation, a 900-second default configurable timeout, and `run --all` with an optional `--task` filter. Failed or incomplete runs are retained and later task runs continue.
- Stage 2 live batch verification completed with only `llama-hase/qwen3.8-27b` (`llama.cpp`): CPP-001, CPP-003, and CPP-005 all returned `SUCCESS` from OpenCode and from independent visible/hidden validation. Agent durations were 424.48 s, 548.70 s, and 324.64 s respectively. The Windows OpenCode adapter uses the npm `.cmd` wrapper and redirects `TEMP`/`TMP` into the workspace.
- Reproducible development environment: Python 3.12.4 is required exactly; `tools/setup-env.ps1` creates/reconciles the ignored repository-local `.venv`, supports a clean `-Recreate`, installs exact packaging/test versions, installs the project editable, and runs CLI/pytest sanity checks.
- Autonomous run reporting: each task now prints the same concise build/visible/hidden/final status layout as validation, and every single or batch run writes a Git-ignored Markdown table under `results/`.
- Autonomous telemetry: OpenCode JSON event streams now provide maximum observed context tokens, generated tokens, estimated model time (step time less recorded tool time), and generated-token rate. Console, Markdown reports, and run metadata also record agent elapsed time and full benchmark elapsed time. The default autonomous timeout is now 1,800 seconds (30 minutes).
- Autonomous OpenCode runs accept an optional `--variant` and pass it through as OpenCode's model reasoning-effort `--variant` (for example, `medium` or `xhigh`), while retaining it in metadata and Markdown reports.
- Stage 3 started: CPP-002 CSV parser added as version 2. It is a bounded C++20 parsing/state-machine task with separate visible and hidden tests for quoted fields, escaped quotes, empty fields, LF/CRLF records, embedded newlines, malformed quoting, and a final record without a newline.
- CPP-004 Thread Pool added as version 2. It is a hard C++20 concurrency/template task with visible and independent hidden tests for futures, exceptions, draining shutdown, fixed workers, concurrent submissions, move-only callables/arguments, and lvalue callables/arguments.
- CPP-004 strengthened to version 3: independent hidden validation now confirms that destruction, as well as explicit shutdown, drains all accepted work. This is a small contract-backed hardening rather than a new shutdown-concurrency requirement.
- CPP-005 strengthened to version 3 after suite review. Its hidden validation now covers `Capacity == 1`, failed-pop output preservation, move-only object lifetime through pop/wrap/destruction, and deterministic producer/consumer transfer of a structured move-only payload with sequence and integrity fields. A narrow source-contract test also enforces its explicit no-standard-mutex and acquire/release requirements. The contract documents that destruction must not overlap producer/consumer use.
- Added `TASKS.md`, the maintained task catalogue for all implemented and planned C++ tasks. It records intended scope, difficulty, and hard/very-hard construction policy; `AGENTS.md` now requires it to be kept in sync with task/version changes.
- Reviewed planned CPP-006 through CPP-020 and deepened their specifications in `CODEX_TASK.md` and `TASKS.md`. Hard tasks now have concrete re-entrancy, ownership, multi-file, deterministic-concurrency, or incremental-state requirements; very-hard tasks now require substantive numerical, synchronization, or architecture invariants with objective validation. This is specification work only: no future task project was created or relabeled.
- Added planned CPP-021 SIMD PCA / covariance kernel as an expert numerical-systems benchmark. It separates scalar correctness from optional runtime-dispatched AVX2 acceleration, requires portable fallback behavior, and defines objective tolerance-based PCA, dispatch, and residual validation. No CPP-021 starter project has been created yet.

## Benchmark specification review (2026-09-11)

- CPP-004 gap incorporated after autonomous model-run review: the original version 1 wording said that `submit(f, args...)` accepted a callable, but did not explicitly require ordinary named (lvalue) callables and arguments. This is expected behavior for the public forwarding API, so version 2 now states it explicitly and an independent hidden regression compiles and invokes a named callable with a named argument. The reference implementation passes version 2.
- CPP-004 autonomous review: `lmstudio/qwen/qwen3.6-35b-a3b` completed its agent run and passed visible tests, but hidden validation could not compile its final wrapper with a move-only argument because it invoked stored arguments as lvalues. Its version-1 and version-2 result is `COMPILATION_FAILURE`. `llama-hase/qwen3.8-27b` passed all version-1 validation using a type-erased promise-based design, but its decayed task constructor cannot accept a named callable; its preserved version-1 workspace correctly fails the new version-2 lvalue regression as `COMPILATION_FAILURE`. A fresh 27B version-2 run then revised its wrapper to own and rvalue-apply a tuple of arguments, and passed both authoritative suites (`SUCCESS`; 709.65 s estimated model time, 784.56 s full time). All generated workspaces are retained. Concurrent overlapping calls to `shutdown()` are explicitly outside CPP-004's scope; sequential repeated shutdown is required and tested.

- CPP-002 gap incorporated after autonomous model-run review: the original contract did not state the result for a bare `\r` inside a quoted field, despite explicitly rejecting it outside quotes. The contract now makes it quoted field data, preserved as `\r`; an independent hidden regression covers it. CPP-002 version increased from 1 to 2.
- CPP-002 autonomous review: `lmstudio/qwen/qwen3.6-35b-a3b` used its full 900-second budget while iterating on a substantial parser, but its preserved result failed visible quoted-field handling and hidden CRLF/final-record coverage. `llama-hase/qwen3.8-27b` passed version 1 visible and hidden validation; it then exposed the quoted-bare-`\r` ambiguity and correctly fails only the version 2 hidden regression. Both generated workspaces are retained under `work/`.

- CPP-001 gap incorporated: “whitespace” now explicitly means the characters recognised by C++ `std::isspace`; visible and hidden tests cover tabs and newlines. The formal decimal grammar now covers integer, trailing-decimal-point, leading-decimal-point, and ordinary decimal forms, with hidden malformed-literal tests. Unary operators around parenthesised expressions and signed-zero division are covered. Scientific/exponent notation, locale-specific input, and non-finite literals are explicitly outside scope and are not scored. CPP-001 version increased from 1 to 2.
- CPP-003 gap incorporated: the cache is formally non-copyable and non-movable, avoiding unspecified ownership transfer and copied internal iterators. Hidden tests enforce those traits, rvalue-key insertion/replacement, and move-only-value replacement. Returned-pointer stability beyond the next non-const operation is explicitly outside scope; no internal iterator API exists. CPP-003 version increased from 1 to 2.
- CPP-005 retained the existing deterministic fixed-count SPSC FIFO stress test and acquire/release requirement. Its contract now explicitly requires construction before raw-storage use and exactly-once destruction, including remaining queued elements. A deterministic lifetime-counter hidden regression covers destruction of an unpopped element; the visible `unique_ptr` case remains to expose the observed assignment-into-unconstructed-storage crash. CPP-005 version increased from 1 to 2.

## Stage 1 verification (2026-09-10)

- Toolchain detected: Visual Studio Community 2026 / MSVC 19.51.36257 x64
  (`cl.exe` at `C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\HostX64\x64\cl.exe`), CMake 4.4.3, Python 3.12.4.
- Framework tests passed: `python -m pytest` (17 tests at the latest verification, inside the recreated `.venv`).
- Commands exercised:
  - `python -m hasebench list`
  - `python -m hasebench info cpp_001`
  - `python -m hasebench prepare cpp_001`, `python -m hasebench prepare cpp_003`, and `python -m hasebench prepare cpp_005`
  - `python -m hasebench validate <workspace> --verbose` for each solved workspace
  - `python -m hasebench run --help` and framework-isolated autonomous-run tests for CPP-001, CPP-003, and CPP-005 (using the same runner/orchestration path without consuming a configured model run)
  - Live autonomous CPP-001 run: `python -m hasebench run cpp_001 --agent opencode --model llama-hase/qwen3.8-27b --backend llama.cpp --label qwen38-27b-q4 --timeout 900` completed `SUCCESS` (264.09-second agent run; independent visible and hidden validators passed).
  - Live autonomous batch: `python -m hasebench run --all --agent opencode --model llama-hase/qwen3.8-27b --backend llama.cpp --label qwen38-27b-q4 --timeout 900` completed successfully for CPP-001, CPP-003, and CPP-005; each used a fresh workspace and the shared authoritative validator.
  - `ctest --test-dir <cpp_005-workspace>\build\hasebench-hidden -C Debug -R ^spsc_hidden$ --output-on-failure --repeat until-fail:10`
- Fresh unmodified starters for all three tasks configured and built with MSVC, then correctly reported `VISIBLE_TEST_FAILURE`.
- Correct local-only solutions passed visible and hidden validation for all three tasks. The canonical task directories were not used as workspaces or modified by preparation/validation.
- Failure classifications verified with intentional isolated-workspace defects: `COMPILATION_FAILURE`, `VISIBLE_TEST_FAILURE`, and `HIDDEN_TEST_FAILURE`.
- Problems found and fixed:
  - duplicate case-insensitive Windows environment-variable names (`PATH` / `Path`) could make MSBuild fail before compilation; validator subprocess environments now deduplicate them;
  - hidden validator targets omitted their required C++20 setting;
  - CTest needed `-C Debug` for Visual Studio's multi-configuration generator;
  - CPP-003's hidden test duplicated visible coverage; it now checks independent capacity-one and access-order cases;
  - CPP-005's hidden test has an explicit 30-second test limit; its fixed-workload concurrency check passed ten consecutive runs.

## Active

- Stage 3 — Expand the C++ suite incrementally. CPP-005 version 3 hardening is complete and reference-quality validation passes; stop here before adding CPP-006.

## Pending

- Stage 3 — Expand the C++ suite.
- Stages 4–9 — Reporting, Pi, Rust, general tasks, telemetry, and visual tasks.
