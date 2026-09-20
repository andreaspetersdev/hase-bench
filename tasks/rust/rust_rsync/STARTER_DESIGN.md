# Modular starter design

The unpublished starter will be one Rust 2024 binary crate with a small library
surface. Modules own narrow responsibilities and must not invoke each other by
shell command.

| Module | Responsibility | Boundary invariant |
| --- | --- | --- |
| `cli` | Parse local, remote-shell, and daemon syntax and supported options | No filesystem access; Windows drive/UNC syntax is classified before remote syntax. |
| `session` | Transfer state machine, role negotiation, cancellation, diagnostics | One terminal outcome; cleanup runs for every exit path. |
| `planner` | Walk, filter, quick-check, deletion plan, hard-link groups | Paths are normalized relative paths confined to a typed root. |
| `fs` | Cross-platform metadata and atomic file operations | Platform adapters expose typed supported, probe-required, host-unsupported, or adapter-unavailable outcomes and return explicit errors for unhonored requests. |
| `delta` | Rolling/strong signatures and literal/match instruction stream | Bounded buffers; reconstructed byte count and digest are verified. |
| `wire` | Protocol-31 framing, multiplexed messages, feature negotiation | Lengths are bounded before allocation; malformed peers cannot escape session roots. |
| `transport::local` | In-process sender/receiver plumbing | Uses the same planner/delta path as network modes. |
| `transport::shell` | Configured remote-shell process and server role | Argument arrays only; no concatenated shell command from remote paths. |
| `transport::daemon` | Client, listener, modules, authentication, access policy | Module roots remain confined; daemon authentication is not treated as encryption. |
| `config` | Filter files, daemon configuration, environment policy | Includes have bounded depth and deterministic precedence. |
| `manifest` | Test-only observable tree/metadata representation | Stable ordering and typed capability annotations. |

The starter will include `IMPLEMENTATION_PROGRESS.md`, CLI entry points, typed
errors, protocol codecs with strict bounds, and failing visible smoke tests for
all three modes. It will not include a local-copy implementation disguised as
the network architecture.

The future author reference and validator are separate trees. Validator code
may launch a configured clone executable and a discovered compatible upstream
rsync, but neither executable controls validator paths, commands, timeouts, or
expected-result locations.
