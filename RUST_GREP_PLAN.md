# `rust_grep`: staged task plan

## Goal and scope

Create one Very-high, long-horizon Rust benchmark project for a practical
grep-like command-line search utility that runs natively on Windows and Linux.
This is a whole-tool agent task, not a single regex function. The ID
`rust_grep` is published at version 1 after the contract, author reference,
and independent validator passed on Windows and Ubuntu WSL2, subject to the
documented Windows capability skips.

The tool must search stdin, explicit files, and recursive trees. Its pinned
contract will cover multiple patterns and pattern files; fixed-string and
documented regex modes; case inversion and whole-word/whole-line matching;
line numbers and filename prefixes; count, matching-file, quiet, and
only-matched output; bounded context lines; include/exclude rules; binary data;
and distinct match, no-match, and error exit codes. State exact option
precedence, output ordering, byte/encoding policy, empty-pattern behavior,
newline handling, and the interaction of quiet mode with I/O errors.

Use an installed GNU grep release as an independent oracle for the common
behavior subset. Record its exact version with each differential run. The
contract is an explicitly pinned grep-like interface, not an implicit promise
to implement every GNU extension. Resolve BRE/ERE and backreference support
before version 1; unsupported syntax must produce a defined diagnostic rather
than a silent change of meaning.

## Platform rules and design review

- Linux: byte-oriented filenames and input, permissions, symlinks, and
  non-UTF-8 paths need explicit outcomes.
- Windows: drive and UNC paths, case-preserving filenames, CRLF files,
  reparse points, console output encoding, and filesystem capability errors
  need explicit outcomes.
- Streaming: bound memory independently of input file size, pattern count,
  and context window. Define behavior for very long lines and zero bytes.
- Traversal: define `-r`/`-R` link following, loop detection, directory
  ordering, unreadable entries, and whether an error can coexist with matches.
- Output: define deterministic byte output, context group separators, match
  spans, and broken-pipe handling. Quality and scan throughput are separate
  benchmark measurements.

Before starter creation, write a behavior matrix linking each option and
platform rule to a module, visible test, independent hidden or differential
test, and capability exception. Review the CLI grammar, matcher, traversal,
streaming, output, and error model together. Pin Rust edition/toolchain and
dependency lockfile. A regex dependency is allowed only if its semantics
match the written contract; the validator must not reuse the candidate's
matcher as its oracle.

## Authoring checkpoints

| Phase | Work | Required evidence |
| --- | --- | --- |
| 0 | Pin the grep baseline, behavior matrix, platform policy, and fixtures. | **Complete:** Windows and Ubuntu WSL2 GNU grep 3.11 smoke probes pass; Linux symlink/non-UTF-8 capabilities are recorded. Rust 1.98.1 is installed, and `/usr/bin/cc` was found on 2026-10-04. |
| 1 | Build modular starter and author reference. | **Scaffold complete:** both build with a shared regex 1.13.1 lockfile on Windows and Ubuntu WSL; three visible smoke tests pass on the reference and two fail as intended on the starter. Four GNU grep 3.11 differential smoke cases pass on each platform. The full contract and independent validator are Phase 2–4 work. |
| 2 | Patterns and streaming matches. | **Complete:** `PHASE2_CONTRACT.md` pins fixed-byte and portable ERE behavior, empty/multiple patterns, ASCII boundaries, binary transition, invalid UTF-8, and the exact 16 MiB line limit. Seven visible tests and 33 independent pattern/streaming cases pass on Windows and Ubuntu WSL, including three GNU grep 3.11 differential cases. A one-byte-buffer unit test checks read-boundary independence. |
| 3 | Recursive selection and output. | **Complete:** `PHASE3_CONTRACT.md` pins fixed mode precedence, non-overlapping spans, bounded context/groups, filename/line formatting, sorted recursive traversal, include/exclude glob rules, and error/quiet ordering. The reference passes 12 visible tests and 34 independent Phase 3 cases on Windows, 36 on Ubuntu WSL; the two link cases are capability-skipped on Windows. Four cases compare against GNU grep 3.11. The starter builds and fails 11 visible behavior tests on both OS families. |
| 4 | Platform and failure review. | **Complete:** `PHASE4_CONTRACT.md` pins native path bytes, access failures, read/output errors, and broken-pipe precedence. Windows drive/extended-path and exclusive-share fixtures pass; Linux non-UTF-8 path and permission fixtures pass. UNC and Windows directory-link cases remain named capability skips. Injected read and broken-writer unit tests, real broken-pipe subprocess tests, and a 96 MiB bounded-memory probe pass on both platforms. Phase 1–3 fixtures and GNU grep 3.11 differentials pass again. |
| 5 | Publication review. | **Complete:** version 1 metadata and the agent-visible `TASK.md` cover the pinned contract. `validator/validate.py` runs independent black-box fixtures outside the workspace; the Rust validator adapter supports executable tasks without changing library-task behavior. A clean starter workspace builds but fails visible/hidden checks on Windows and Ubuntu WSL2; the author reference passes 12 visible tests and the full hidden suite on both. The default 1,800-second agent budget is insufficiently justified for this very-high whole-tool task; use `--timeout 3600` for initial scored trials and record the setting. Windows UNC and directory-link coverage are explicit capability exceptions, not passes. |

Keep the canonical starter separate from hidden fixtures. Run each autonomous
attempt in a fresh workspace, preserve logs, and use the Rust executable
validator. Version 1 is published; changing the task contract or tests now
requires a version increment.
