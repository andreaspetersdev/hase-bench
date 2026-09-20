# Platform and filesystem rules

Every rule produces one of `required`, `capability-dependent`, or
`unsupported-with-diagnostic`. A capability-dependent case must be probed and
reported; it must not disappear as a skipped test.

| Area | Linux | Windows | Required outcome |
| --- | --- | --- | --- |
| Path roots | `/` and relative paths | drive roots, drive-relative paths, UNC roots, and extended-length input | Parser distinguishes local roots from `host:path`; a drive colon is never a remote separator. |
| Separators | `/` | accept `\` and `/` at CLI boundary | Normalize internally without losing a UNC prefix or trailing-source-slash semantics. |
| Case | Probe filesystem, normally sensitive | Probe filesystem, normally insensitive/preserving | Detect destination collisions before writing; report a deterministic conflict. |
| Timestamps | Preserve representable seconds/nanoseconds | Preserve representable Windows file times | Compare at the measured filesystem resolution; out-of-range values produce a diagnostic. |
| Permissions | Preserve mode with `-p`; apply umask otherwise | Map writable/read-only where representable | Unsupported Unix mode bits are capability-dependent and reported in the manifest. |
| Ownership | Preserve uid/gid only when permitted | No implicit SID↔uid mapping | Lack of privilege is an explicit partial-transfer/metadata error, not silent success. |
| ACLs | Preserve with `-A` when rsync and filesystem advertise ACLs | Preserve only through the Windows adapter when supported | Probe first; unsupported requests fail with a feature diagnostic. |
| Extended attributes | Preserve with `-X` when advertised | Preserve named streams/xattrs only when adapter and filesystem advertise support | Probe first and compare names plus opaque bytes. |
| Symlinks | `-l`, `-L`, safe-links behavior required | Native symlink behavior requires host privilege/capability | Unsupported creation is reported; never silently materialize a different object. |
| Hard links | `-H` required when supported | Required on supporting NTFS/ReFS volumes | Compare file identity groups, not only bytes. |
| Reparse points | Not applicable beyond symlink-like mounts | Never traverse unknown reparse points by default | Known symlinks follow selected link policy; unknown tags yield a diagnostic. |
| Sparse files | Preserve holes with `-S` where measurable | Preserve sparse allocation when volume supports it | Bytes must always match; allocation reduction is capability-dependent and measured. |
| Special files | Devices/FIFOs require privilege and explicit options | Unsupported | Refuse unsupported creation deterministically; never replace it with a regular file. |
| Long names | Respect component/path limits | Use long-path-safe APIs internally | Preflight impossible destination names and avoid partial unsafe writes. |
| Reserved names | Ordinary filesystem rules | Reject device names and invalid trailing dot/space aliases | Report the original relative path and collision category. |
| Atomic replace | Rename within destination filesystem | Replace using Windows sharing-safe adapter behavior | Temporary files stay in the destination filesystem unless explicitly configured. |
| File locking | Advisory/ordinary open failures | Sharing violations are expected | Retry only under documented timeout policy, then classify as file-I/O failure. |
| Delete safety | Root confinement and filter protection required | Same, including drive/UNC roots and reparse points | Canonicalized deletion target must remain beneath the destination root. |

Authoritative manifests record bytes, entry kind, normalized relative path,
size, modification time at probed resolution, mode/attributes, link target,
hard-link identity group, sparse allocation when supported, ACLs, and xattrs.
