# RUST-GREP platform policy (Phase 0)

## Shared rules

All tests redirect stdout and stderr to byte-capturing files.  Output ordering
is therefore independent of console code pages.  File contents are processed
as bytes; only regex-mode text requires UTF-8.  An unreadable input is a
diagnostic and exit code 2 unless quiet mode has already found a match.

## Windows

Paths are accepted through Rust's native `OsString` API and shown with
`Path::display()`.  Drive-qualified and UNC paths are legal explicit inputs;
the test suite will use a temporary drive path and will capability-skip a UNC
share when none is configured.  A reparse-point directory is not traversed by
`-r`; `-R` traverses it only when its target identity can be obtained and
stops on a detected loop.  CRLF is normalized solely for logical-line
matching, not for byte output.  ACL/share violations are tested through a
capability fixture and are not faked as POSIX permission bits.

Observed 2026-10-03: `C:\` and `\\?\C:\` are available.  The probe for
`\\server\share` was denied, so UNC behavior remains capability-pending.

## Linux

Names and contents are byte-oriented.  Non-UTF-8 path names are valid for
fixed-mode traversal and diagnostics may use an escaped representation; they
are skipped with an explicit diagnostic in regex mode.  `-r` does not follow
symlinks; `-R` follows symlinked directories only after inode/device loop
detection.  Unreadable-file behavior is tested with a non-root account, not
from a privileged CI process.

On 2026-10-03, the Ubuntu WSL2 runner passed the GNU grep 3.11 smoke probe and
confirmed both symlink target identity and non-UTF-8 filename creation. Rust
1.98.1/Cargo 1.98.1 are installed. A 2026-10-04 check found `/usr/bin/cc`,
`/usr/bin/gcc`, and `/usr/bin/ld`; the previous linker block has cleared.
