# RUST-RSYNC — Cross-platform rsync clone (draft)

This contract is not published yet. The directory intentionally has no
`task.yaml`; Phase 0 review may still change APIs and tests.

Implement a Rust 2024 command-line program named `rust-rsync` that provides an
rsync 3.x-compatible client and server on Windows and Linux. Protocol 31 is the
minimum network baseline. The final implementation must support local,
remote-shell, and daemon transfers in both directions and interoperate with a
compatible upstream rsync 3.x peer.

The complete scored surface is maintained in `COMPATIBILITY_MATRIX.csv`.
Platform behavior follows `PLATFORM_RULES.md`. An unsupported host capability
must produce the documented diagnostic/status and capability record; silently
skipping a requested feature is incorrect.

The final task will require the module boundaries in `STARTER_DESIGN.md`, a
workspace `IMPLEMENTATION_PROGRESS.md`, bounded parsing/allocation, root-safe
path handling, independent visible and hidden validation, and a larger
task-specific run budget. This draft becomes `TASK.md` only after the author
reference and cross-platform validator pass the Phase 0 review gate.
