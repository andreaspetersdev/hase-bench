# RUST-GREP Phase 4 platform and failure contract

This is the Phase 4 authoring record. Version 1 incorporates its rules in
`TASK.md` and its black-box fixtures in `validator/validate.py`.

## Native paths and access failures

- On Linux, fixed mode accepts arbitrary native path bytes. A filename prefix
  writes those original bytes, including non-UTF-8 bytes; recursive traversal
  retains the same spelling. Regex mode rejects a non-UTF-8 input path with a
  diagnostic and exit code 2. Diagnostic path rendering may be lossy.
- On Windows, explicit drive-qualified and extended-length paths retain their
  supplied spelling in filename prefixes. UNC paths are legal but require an
  accessible share fixture; the current workstation has none. Redirected
  output is UTF-8. Console code-page conversion is outside the test contract.
- An unreadable input produces a diagnostic and exit code 2. Linux tests use
  mode-zero files under a non-root account; Windows tests deny sharing with
  an exclusive `CreateFileW` handle. Neither condition is inferred from file
  mode bits on the other platform.
- Recursive `-r` does not follow directory links. `-R` follows them and
  detects loops using `(device, inode)` on Unix and canonical target paths on
  Windows. Windows directory-link creation is unavailable in this test
  environment, so the Windows link fixture remains capability-skipped and
  must be reviewed with a suitable fixture before publication.

## Stream and output failures

- A read error retains bytes already written to stdout, reports the failing
  input, and exits 2. An injected reader unit test fails immediately after
  a selected line and verifies that the earlier `hit\n` remains in output.
- A non-broken-pipe write error exits 2. A broken stdout pipe exits 0 only
  when a selected result has been established and no preceding input error
  occurred; otherwise it exits 2. This includes mode outputs such as a zero
  count, which do not establish a selected result. Subprocess fixtures close
  the stdout reader before a large output write to exercise the real pipe.
- Search memory must not grow with input-file size. The reference reads at
  most a 16 MiB logical line plus its bounded context window and pattern
  state. `phase4_memory.py` searches a 96 MiB file of short lines with
  `-B16`, asserts peak RSS <= 64 MiB, and found 6.1 MiB on Windows and
  5.4 MiB on Ubuntu WSL2 in the 2026-10-04 run. This is an author-reference
  acceptance probe, not a throughput score.

## Evidence and remaining capability work

`phase4_platform.py`, `phase4_failures.py`, and `phase4_memory.py` pass on
Windows and Ubuntu WSL2, apart from the named Windows capability skips.
Reference unit tests (3) and visible tests (12) pass on both systems. Phase 1,
2, and 3 fixtures were rerun after the Phase 4 changes; their GNU grep 3.11
differential cases still pass. Phase 5 integrated the black-box fixtures and
documented the Windows directory-link and UNC capability exceptions.
