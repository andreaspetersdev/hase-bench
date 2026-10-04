# RUST-GREP version 1 validator

`validate.py <built-rust_grep-executable>` is the authoritative black-box
hidden suite. The framework builds the candidate from its workspace, runs
the visible Cargo tests, then calls this script with that built executable.
The validator is outside the copied workspace and does not import candidate
matching code. Phase 1–3 fixtures include portable GNU grep 3.11 differential
cases; all exact-byte and platform-specific expectations are independently
specified here. Each fixture has a subprocess timeout and the framework
limits the overall hidden phase to 120 seconds.

On Windows, directory-link tests skip when the workstation cannot create
such links. The UNC fixture skips unless `RUST_GREP_UNC_FIXTURE` names an
accessible share. On Ubuntu WSL2, symlink, non-UTF-8 path, and non-root
permission tests run. Skips must be reported and must not be counted as
passes; comparisons across hosts should record the host capabilities.

The 96 MiB memory fixture is a correctness bound (peak RSS at most 64 MiB),
not a throughput metric. Injected reader/writer tests live in the author
reference because a black-box CLI cannot portably force a read error after
a successful line on both operating systems. The hidden suite separately
tests input-error ordering and real broken stdout pipes.
