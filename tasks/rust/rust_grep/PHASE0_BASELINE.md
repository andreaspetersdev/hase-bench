# RUST-GREP Phase 0 baseline

This is the historical Phase 0 baseline. `rust_grep` was subsequently
published at version 1 with `task.yaml`, starter, reference, and validator.

## Pinned candidate contract

The first published version will be a Rust 2024 executable named
`rust_grep`.  Its portable comparison subset is GNU grep's `-E` mode, with
the option set in `BEHAVIOR_MATRIX.md`.  It is a deliberately smaller,
explicit interface, not a claim of full GNU grep compatibility.

- Regex mode accepts the portable ERE subset implemented by the selected
  Rust regex engine: literals, `.`, bracket classes, `^`, `$`, grouping,
  alternation, and `?`, `*`, `+`, `{m,n}` repetition.  Backreferences,
  look-around, locale-sensitive character classes, and Perl syntax are
  rejected with a usage diagnostic and exit code 2.
- Fixed mode treats a pattern as raw bytes. Regex mode requires each pattern
  and searched line to be valid UTF-8; invalid UTF-8 is an error (status 2),
  never silently decoded with replacement.
- Version 1 will support `-e`, `-f`, `-F`, `-E`, `-i`, `-v`, `-w`, `-x`,
  `-n`, `-H`, `-h`, `-c`, `-l`, `-L`, `-q`, `-o`, `-A`, `-B`, `-C`, `-r`,
  `-R`, `--include`, and `--exclude`.  Any other option is a usage error.
- Exit statuses are 0 (one or more selected lines), 1 (no selected lines),
  and 2 (usage or I/O error).  `-q` may return 0 on its first match even if a
  later input would fail; an error encountered before a match returns 2.

The exact crate versions and `Cargo.lock` are intentionally deferred until
Phase 1, when both Windows and Linux can resolve the same dependency graph.
The language/toolchain baseline is Rust 1.98.1 / Cargo 1.98.1, observed on
Windows 2026-10-03.  Phase 1 must pin its dependency versions and commit the
lockfile before the starter is exposed.

## GNU grep oracle

Windows evidence was collected on 2026-10-03:

```text
grep.exe (GNU grep) 3.11
patched by: Michael M. Builov <mbuilov@yandex.ru>
grep -P uses PCRE2 10.42 2022-12-11
```

The source was `C:\Users\abo\scoop\shims\grep.exe`.  Differential fixtures
must record `grep --version` in their artifact and compare only the portable
subset above.  The oracle is never linked into, copied into, or used by the
candidate process.

The Phase 0 probe also observed that this Windows GNU grep port emits CRLF
when writing matched lines, including for an LF-only input.  Accordingly,
Windows differential fixtures compare status, selected records, and
normalized line endings; exact output bytes are checked against the
`rust_grep` contract separately.  Linux fixtures may compare the contract's
LF bytes directly.

## Phase 0 status

Windows and Linux GNU-grep oracle fixtures pass.  Linux evidence was captured
on 2026-10-03 through Ubuntu on WSL2 (`Linux 6.6.87.2-microsoft-standard-WSL2`,
Python 3.12.3, GNU grep 3.11).  The Linux filesystem probe confirms symlink
target identity and non-UTF-8 filename support.  This runner has neither
`rustc` nor `cargo`, so it cannot build the future candidate/reference.

The grep-oracle and filesystem portion of Phase 0 is complete. Rust 1.98.1
and Cargo 1.98.1 were subsequently installed in this WSL runner. A new check
on 2026-10-04 found `/usr/bin/cc`, `/usr/bin/gcc`, and `/usr/bin/ld`, clearing
the previous linker block. Phase 1 may now generate a shared lockfile and
build on both operating systems.
