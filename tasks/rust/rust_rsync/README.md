# RUST-RSYNC Phase 0

This directory contains design and validator-fixture material for the planned
`rust_rsync` capstone. It deliberately has no `task.yaml`, so task discovery
does not expose an unfinished benchmark.

The compatibility target is rsync 3.x with protocol 31 or newer. An exact
package build is not part of the task contract. Every authoritative run must
still record the reference executable's version, protocol, capabilities, and
host environment so results can be reproduced and interpreted.

Phase 0 artifacts:

- `BASELINE.md` defines compatibility and exit-status boundaries.
- `PLATFORM_RULES.md` defines Windows/Linux filesystem behavior.
- `COMPATIBILITY_MATRIX.csv` maps behavior to implementation and tests.
- `STARTER_DESIGN.md` defines the future Rust module boundaries.
- `validator/fixture_probe.py` exercises a reference rsync in isolated local,
  loopback remote-shell, and loopback daemon modes.
- `validator/differential_fixture.py` compares candidate and oracle roles.
- `validator/windows_capabilities.py` records structured filesystem capability
  outcomes for links, read-only mapping, named streams, sparse ranges, ACL
  inspection, xattr APIs, ownership mapping, timestamps, case, and long paths.
- `validator/windows_wsl_local_diff.py` compares native local output with WSL rsync.
- `starter/` is the compiling unpublished Rust module skeleton.
- `reference/` is the in-progress author reference; its README lists gaps.

Run the current Linux/WSL fixture from the repository root:

```text
wsl.exe python3 /mnt/d/project/hase-bench/tasks/rust/rust_rsync/validator/fixture_probe.py
```

The fixture creates all state in a temporary directory and binds the daemon
only to loopback. It does not install or download rsync.
