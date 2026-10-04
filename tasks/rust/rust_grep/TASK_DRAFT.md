# rust_grep authoring draft — superseded by TASK.md

Implement a native Rust 2024 command-line search utility for Windows and
Linux. The first release will use the pinned CLI, matching, streaming,
traversal, output, and error rules in `PHASE0_BASELINE.md`,
`BEHAVIOR_MATRIX.md`, `PHASE2_CONTRACT.md`, `PHASE3_CONTRACT.md`,
`PHASE4_CONTRACT.md`, and
`PLATFORM_POLICY.md`. The candidate receives only
the starter and its visible tests; independent differential and hidden
fixtures stay outside its workspace.

Version 1 is published in `TASK.md` and `task.yaml`. This file remains only
as authoring history; it is not copied into agent workspaces. The independent
validator is `validator/validate.py`.
