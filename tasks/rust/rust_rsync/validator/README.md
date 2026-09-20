# Phase 0 validator fixture

`fixture_probe.py` verifies that an already-installed rsync 3.x/protocol-31+
oracle can support the three mandatory fixture modes. It is intentionally an
oracle self-test, not yet the final clone validator.

The probe creates a source tree containing text, binary data, an empty file, a
name with spaces, and (when available) a relative symlink. It compares stable
tree manifests after:

1. local archive transfer;
2. loopback remote-shell push and pull using a temporary shell shim;
3. loopback daemon push and pull using a temporary writable module.

The smoke module explicitly disables daemon symlink munging so its tree can be
compared byte-for-byte with the local and remote-shell results. The eventual
daemon security suite separately tests the safe default that munges symlinks
uploaded to writable modules.

The JSON result records the exact rsync banner and each mode's status. A
missing binary, non-3.x release, protocol below 31, mode failure, or manifest
mismatch is a hard failure. Host metadata capabilities are handled later by
the matrix-specific fixtures and are never inferred from this smoke probe.

`differential_fixture.py` accepts separate `--candidate` and `--oracle`
executables. It checks local archive/delete/dry-run behavior, both client and
server roles for remote-shell push/pull, and both client and server roles for
daemon push/pull. Its self-check uses `/usr/bin/rsync` for both roles; the
unfinished Rust starter is expected to fail when supplied as the candidate.

`windows_capabilities.py` records case behavior, timestamp representation,
hard-link and symlink creation, long-path behavior, and explicit deferred
capabilities. It creates only temporary probe entries and requires no elevated
privileges.

`windows_wsl_local_diff.py` runs a native Windows candidate and WSL rsync over
separate trees in one Windows temporary directory. It independently compares
archive contents, deletion, dry-run immutability, source trailing-slash and
destination-shape rules, multiple sources, binary bytes, and names containing
spaces. Drive paths are translated deterministically to `/mnt/<drive>/...`;
no shell command is constructed from fixture paths.
