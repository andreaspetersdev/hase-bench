# Hase Bench progress

## Completed

- Repository layout, task discovery, workspace preparation, and framework unit tests.
- CPP-001 Expression Evaluator, CPP-003 Generic LRU Cache, and CPP-005 SPSC Ring Buffer canonical tasks, each with visible and separate hidden tests.
- Stage 1 — Framework skeleton verified end-to-end with real MSVC for CPP-001, CPP-003, and CPP-005.
- Manual workflow: `validate --all` discovers only metadata-marked direct children of `work/`, supports `--task`, continues after corrupt workspaces, and prints per-run plus aggregate outcomes.
- Manual preparation supports `--label` for an optional safe model/run suffix; new workspace names use the concise `_man` marker.
- Benchmark-specification review of CPP-001, CPP-003, and CPP-005 based on real manual model runs; all three task versions are now version 2.

## Benchmark specification review (2026-09-11)

- CPP-001 gap incorporated: “whitespace” now explicitly means the characters recognised by C++ `std::isspace`; visible and hidden tests cover tabs and newlines. The formal decimal grammar now covers integer, trailing-decimal-point, leading-decimal-point, and ordinary decimal forms, with hidden malformed-literal tests. Unary operators around parenthesised expressions and signed-zero division are covered. Scientific/exponent notation, locale-specific input, and non-finite literals are explicitly outside scope and are not scored. CPP-001 version increased from 1 to 2.
- CPP-003 gap incorporated: the cache is formally non-copyable and non-movable, avoiding unspecified ownership transfer and copied internal iterators. Hidden tests enforce those traits, rvalue-key insertion/replacement, and move-only-value replacement. Returned-pointer stability beyond the next non-const operation is explicitly outside scope; no internal iterator API exists. CPP-003 version increased from 1 to 2.
- CPP-005 retained the existing deterministic fixed-count SPSC FIFO stress test and acquire/release requirement. Its contract now explicitly requires construction before raw-storage use and exactly-once destruction, including remaining queued elements. A deterministic lifetime-counter hidden regression covers destruction of an unpopped element; the visible `unique_ptr` case remains to expose the observed assignment-into-unconstructed-storage crash. CPP-005 version increased from 1 to 2.

## Stage 1 verification (2026-09-10)

- Toolchain detected: Visual Studio Community 2026 / MSVC 19.51.36257 x64
  (`cl.exe` at `C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\HostX64\x64\cl.exe`), CMake 4.4.3, Python 3.12.4.
- Framework tests passed: `python -m unittest discover -s tests -v` (5 tests).
- Commands exercised:
  - `python -m hasebench list`
  - `python -m hasebench info cpp_001`
  - `python -m hasebench prepare cpp_001`, `python -m hasebench prepare cpp_003`, and `python -m hasebench prepare cpp_005`
  - `python -m hasebench validate <workspace> --verbose` for each solved workspace
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

- No active implementation stage. Stage 1 is complete.

## Pending

- Stage 2 — Autonomous OpenCode execution (explicitly out of scope until Stage 1 is reliable).
- Stage 3 — Expand the C++ suite.
- Stages 4–9 — Reporting, Pi, Rust, general tasks, telemetry, and visual tasks.
