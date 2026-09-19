# `rust_rsync`: staged task plan

## Goal and boundary

Build one Rust benchmark project whose deliverable is a full functional rsync
clone on Windows and Linux. It must interoperate with a pinned upstream rsync
release in local, remote-shell, and daemon modes. The project is a long-horizon
capstone after the generic Rust task pipeline works. Implementation phases are
checkpoints inside one task, not separate tasks with reduced final scope.

Before publishing version 1, record the exact upstream release, protocol
versions, `rsync(1)` and `rsyncd.conf(5)` manuals, Rust toolchain, dependency
lockfile, supported Windows/Linux versions, and filesystem test environments.
Create an option-and-behavior matrix from that pinned documentation. Each row
must identify the implementation module, visible test, hidden or differential
test, and any explicit host-capability rule. The final pass criterion is the
whole matrix, not a selected subset. Changes to the matrix, starter, or
authoritative tests after comparable results exist require a task version bump.

## Phase 0 — Benchmark design and validation fixture

1. Define CLI and wire compatibility against the pinned upstream release,
   including direction (push/pull), error and exit semantics, and daemon config.
2. Record platform rules for drive letters and UNC paths, case sensitivity,
   timestamps, permissions, ownership, ACLs, extended attributes, links,
   reparse points, sparse files, and unsupported filesystem capabilities.
3. Design a modular starter with CLI, transfer planner, file-system adapter,
   delta engine, wire protocol, remote-shell transport, and daemon modules.
4. Build isolated fixtures for local transfers and loopback remote-shell/daemon
   transfers. Pin an upstream rsync binary in the validator environment.
5. Write the author reference and independent tests before publishing the
   task. Verify the clean starter fails meaningfully and the reference passes
   on both operating systems.

Review gate: the option matrix, platform rules, reference, and test fixtures
must be reviewed together. An unsupported host capability has a documented
outcome; it is not treated as a silently skipped feature.

## Phase 1 — Agent design

The agent records a short design in the workspace before implementation:
module boundaries, transfer state machine, path and metadata representation,
memory/streaming limits, protocol framing, temporary-file/atomic-update
strategy, failure recovery, and how each platform adapter meets the matrix.
The design identifies testable invariants and risks. Keep the design file in
the workspace so the run can be reviewed without reading the agent log.

Review gate: the design accounts for all three transfer modes and both OS
families. No implementation is accepted as complete from a local-copy-only
architecture.

## Phases 2–7 — Implementation checkpoints

| Phase | Implementation | Required checkpoint evidence |
| --- | --- | --- |
| 2. CLI and local transfer | Parse pinned rsync syntax and options; classify Windows paths correctly; traverse trees; copy files and directories; apply quick-check and checksum decisions. | Local differential fixtures for source/destination syntax, dry run, unchanged files, binary data, long and unusual names, and error exits. |
| 3. Selection and filesystem behavior | Implement include/exclude/filter rules, deletion timing, links, hard links, sparse files, metadata, and platform capability mapping. | Tree manifest and metadata comparisons; protection against traversal and symlink escapes; Windows and Linux filesystem cases. |
| 4. Delta and recovery | Implement rolling-signature matching, literal/matched blocks, streaming transfer, partial files, interrupted resume, and atomic replacement where specified. | Byte-for-byte randomized/adversarial comparisons, reduced transferred bytes for small edits, injected interruption and retry, bounded-memory checks. |
| 5. Remote-shell mode | Implement push/pull via a configured remote shell and the pinned rsync wire protocol; support this clone talking to upstream rsync in both directions. | Loopback and cross-host clone↔upstream tests, protocol errors, disconnects, and path quoting. |
| 6. Daemon mode | Implement daemon client/server operation, module configuration, authentication and access rules, and upstream-compatible protocol behavior. | Clone↔upstream daemon tests, module boundaries, failed authentication, malformed requests, and concurrent sessions. |
| 7. Remaining compatibility | Complete every remaining option and behavior in the pinned matrix, including diagnostics and exit status. Remove implementation-only gaps. | Matrix has no unimplemented rows; every capability-based exception has a deterministic test and user-facing behavior. |

At each checkpoint: compile, run relevant visible tests, update the workspace
progress file, and review changed code for API boundaries, resource cleanup,
error propagation, and unsafe path handling. A passing intermediate checkpoint
is progress evidence, not a final benchmark success.

## Phase 8 — Final review and tests

Run the complete visible and independent hidden suites on Windows and Linux.
The validator must exercise local, remote-shell, and daemon transfers; both
clone→upstream and upstream→clone interoperability; and Windows↔Linux paths
where the configured test hosts permit it. Compare final trees, bytes,
metadata, deletions, logs, and exit codes with the pinned upstream release.
Include random and adversarial fixtures, interruption/restart, malformed wire
input, protocol negotiation, permission failures, and timeout containment.
Keep correctness distinct from transfer speed and telemetry.

Review the final diff against the design and option matrix. Record findings,
fixes, and re-runs. Final success requires every applicable row to pass on
both operating systems, with no transfer mode omitted. If the reference or
validator is wrong, correct it and version the task before comparing runs.

## Progress tracking and run rules

- `progress.md` tracks task authoring status: phase, date, completed evidence,
  open gaps, next checkpoint, and task version. Update it when a phase closes
  or the contract changes.
- The starter includes `IMPLEMENTATION_PROGRESS.md` with one row per phase:
  `not started`, `in progress`, `blocked`, or `done`; evidence (test command and
  result); design/review notes; and remaining gaps. The agent updates this
  inside its workspace. Claims of `done` are checked independently.
- Run metadata records the last completed phase and each validation gate's
  result as well as the final classification. Preserve logs and workspace.
- A manual multi-session build stays one workspace and is marked manual or
  assisted. Each autonomous attempt starts from a fresh canonical workspace;
  it may work through the phases in that one run with a task-specific time
  budget. Never carry a prior attempt's edits into a new autonomous run.
- Do not count partial phase completion as overall success. It may be reported
  separately to explain model behavior.

The benchmark framework remains responsible for independent validation; the
agent's own test log and progress file are useful review evidence, not the
authoritative score.
