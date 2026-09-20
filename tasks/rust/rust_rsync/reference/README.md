# Author reference status

This is the in-progress author-only reference, not a published solution. It
depends on the starter crate for the reviewed public parsing and domain types.

Implemented and tested:

- local endpoints;
- explicit source trailing-slash intent;
- recursive regular file/directory copy;
- symbolic-link object preservation with `-l`/archive mode, referent copying
  with `-L`, and lexical transferred-tree confinement with `--safe-links`;
- `--delete` for extraneous destination entries;
- ordered inline include/exclude rules, merge filter files, and excluded-path
  delete protection including `--delete-excluded`;
- inherited per-directory merge rules with directory-relative anchoring;
- hard-link identity groups with `-H` through safe cross-platform handles;
- Windows read-only mapping under archive/permission preservation;
- exit-23 unsupported-capability diagnostics when Windows cannot create a
  requested native symbolic link;
- typed metadata capability reporting for timestamps, permissions, ownership,
  ACLs, xattrs/named streams, symlinks, hard links, and sparse files;
- Windows ACL preservation through a fixed PowerShell command whose paths are
  passed only through environment variables;
- Windows named-stream enumeration with opaque byte copying in Rust;
- Windows sparse output with measured allocation reduction through argument-array
  `fsutil` calls;
- rolling weak block signatures plus pinned SHA-256 0.10.9 strong signatures,
  literal/match planning, and length/digest-verified reconstruction;
- localized-edit block reuse with literal-byte accounting, output bounds, and
  malformed-plan rejection;
- pre-mutation exit-23 diagnostics for explicit metadata requests that the
  selected platform adapter cannot honor;
- mutation-free `--dry-run`;
- regular-file modification times at the exercised rsync/filesystem precision;
- rsync-style broad syntax and partial-transfer exit categories.

The Windows reference is independently differential-tested against WSL rsync
3.2.7 for trailing-slash contents, non-trailing directory naming with absent
and existing destinations, recursive binary/name handling, `--delete`,
`--delete-excluded`, `--dry-run`, ordered and inherited filters, merge files,
timestamps, hard-link identity, and multiple sources. The symlink fixture
records the current host's missing native creation privilege rather than
claiming a skipped pass; where native links are available it compares `-l`,
`-L`, and `--safe-links`. The first differential run corrected an author-test
assumption about the absent-destination directory shape.

Not yet implemented:

- complete option and filter semantics;
- Unix ownership, ACL, xattr, and sparse adapters plus remaining metadata
  mapping;
- streaming delta memory bounds and interruption recovery;
- protocol-31 framing and remote-shell roles;
- daemon client/server, modules, and authentication;
- every remaining row in `COMPATIBILITY_MATRIX.csv`.

Passing these local unit tests is checkpoint evidence only. It is not Phase 0
completion and is not a benchmark success.
