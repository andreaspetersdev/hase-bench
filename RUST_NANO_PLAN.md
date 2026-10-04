# `rust_nano`: staged task plan

## Goal and scope

Create one Very-high, long-horizon Rust benchmark project for a practical
nano-like terminal editor that runs natively on Windows and Linux. Reserve the
ID `rust_nano`. It must be an interactive editor with a real terminal frontend,
not a line-editing library or a scripted text transformer. Its version-1
contract will pin a substantial, testable command set; resemblance to nano
does not imply every GNU nano command or configuration extension.

The contract must cover opening and creating files, multiple buffers,
navigation and viewport scrolling, insertion and deletion, selection,
cut/copy/paste, search and replace, undo/redo, save/write-out, modified-buffer
prompts, status/help UI, terminal resizing, and clean exit. Define exact
bindings and prompt behavior before agent runs. Editing is Unicode-aware:
specify UTF-8 decoding, grapheme navigation, display width, combining marks,
tabs, and line-ending preservation. State what happens with invalid UTF-8,
large files, read-only files, failed writes, external modification, backup
creation, and interrupted saves.

## Architecture and validation rules

- Separate a deterministic document/command model from terminal input,
  rendering, and filesystem adapters. Hidden tests must observe semantic
  state, not infer correctness only from screenshots.
- Run real interaction fixtures through a Unix PTY on Linux and ConPTY or a
  controlled Windows console on Windows. Compare terminal cell snapshots and
  command outcomes at defined sizes; avoid subjective visual judgment.
- Use an independently scripted editing oracle for operation sequences.
  Capture a pinned GNU nano release as a behavioral reference where its
  terminal behavior is portable, recording the exact version per run.
- Define save atomicity and recovery explicitly. A failed or interrupted save
  must not silently destroy the prior file; temporary and backup artifacts
  need deterministic rules. Preserve file permissions and newline style where
  the contract requires them.
- Review terminal restoration after normal exit, errors, and forced
  interruption. Separate correctness from redraw count, latency, and memory.

Before starter creation, write a command/state matrix mapping every binding,
prompt, edit, file operation, and platform rule to visible and independent
tests. Pin Rust edition/toolchain, terminal and Unicode dependencies, and the
lockfile. The starter should expose document, undo history, command dispatch,
screen layout, terminal adapter, and file-store boundaries so the agent must
integrate a complete application.

## Authoring checkpoints

| Phase | Work | Required evidence |
| --- | --- | --- |
| 0 | Pin command set, nano baseline, terminal capabilities, and state matrix. | PTY/ConPTY fixture self-checks and Windows/Linux capability records. |
| 1 | Build modular starter and author reference. | Clean starter fails meaningfully; reference runs on both OS families. |
| 2 | Document and editing semantics. | Unicode/grapheme and newline cases, navigation, selection, clipboard, multi-buffer isolation, deterministic undo/redo. |
| 3 | Filesystem and recovery. | Open/save/write-out, permissions, external changes, failed/interrupted save, temp cleanup, backup behavior. |
| 4 | Interactive terminal behavior. | Key decoding, prompts, scrolling, resize, rendering snapshots, terminal restoration, and end-to-end scripted sessions. |
| 5 | Publication review. | Complete matrix, versioned task contract, independent hidden tests, cross-platform reference validation, and reviewed time budget. |

Keep semantic model tests and real-terminal tests distinct so a candidate
cannot pass by printing expected frames without maintaining correct document
state. Run each autonomous attempt in a fresh workspace and preserve session
logs. No benchmark version exists until the task is published.
