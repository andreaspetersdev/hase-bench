# Compatibility baseline

## Contract

RUST-RSYNC targets the documented, interoperable core of rsync 3.x. Protocol
31 is the minimum wire version. A peer may negotiate a newer protocol, but a
feature introduced after protocol 31 cannot be required unless the matrix
marks it as an optional negotiated capability.

The validator uses an available upstream rsync 3.x executable as a behavioral
and wire oracle. It does not require one exact package checksum. Each run must
capture:

- `rsync --version`, including protocol, capabilities, checksum algorithms,
  compression algorithms, and daemon authentication algorithms;
- executable path and operating-system identity;
- filesystem type and tested capability probes;
- Rust compiler version and the clone's dependency lockfile hash.

The first exercised oracle is Ubuntu/WSL rsync 3.2.7, protocol 31. This is run
metadata, not a permanent restriction on later compatible 3.x validators.

## Documentation

Semantics follow the `rsync(1)` and `rsyncd.conf(5)` manuals shipped with the
exercised 3.x implementation. The upstream 3.2.7 source release is available
from <https://download.samba.org/pub/rsync/src/rsync-3.2.7.tar.gz>. Current
official manuals remain useful for compatible behavior but do not silently add
newer-only requirements:

- <https://download.samba.org/pub/rsync/rsync.1>
- <https://download.samba.org/pub/rsync/rsyncd.conf.5>

If two accepted 3.x implementations differ, the matrix must either select the
protocol-31 behavior or mark the behavior as capability/version-dependent.
That decision is a contract change once version 1 results exist.

## Required modes and directions

The final benchmark requires local copy, remote-shell push and pull, and daemon
push and pull. Clone-to-upstream and upstream-to-clone wire combinations are
required for both network modes. Omitting a transfer mode is never a supported
host-capability exception.

## Errors and exits

The clone must use rsync-compatible process categories. The matrix owns the
exact scenarios for exits 0, 1, 2, 3, 5, 10, 11, 12, 20, 23, 24, 25, 30, and
35. Diagnostics need not reproduce incidental wording byte-for-byte, but must
identify the operation/path and retain the same machine-observable category.
Signals and host-specific spawn errors are normalized by the validator before
comparison.

## Security boundary

Daemon fixtures bind only to `127.0.0.1` on an ephemeral port and use temporary
module roots. Remote-shell fixtures use a local test shim, not a login service.
No fixture exposes a writable module to the LAN or executes paths supplied by
the implementation under test as validator commands.
