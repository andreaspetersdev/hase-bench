# Author reference status

This Rust 2024 executable is the version 1 author reference. It
builds with the shared `Cargo.lock` on Windows and Ubuntu WSL and passes 12
visible tests. The reference passes 33 independent Phase 2 cases on both
platforms, plus 34 Phase 3 cases on Windows and 36 on Ubuntu WSL. Portable
cases are also compared with GNU grep 3.11. Phase 4 native-path, access,
broken-pipe, injected-read, and bounded-memory checks pass on both platforms,
with UNC and Windows directory-link fixtures capability-skipped.

The reference is not copied to agent workspaces. Score only fresh starter
workspaces against the external validator; record Windows capability skips
and use a 3,600-second agent timeout for initial long-horizon trials.
