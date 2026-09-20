# Author reference status

This is the in-progress author-only reference, not a published solution. It
depends on the starter crate for the reviewed public parsing and domain types.

Implemented and tested:

- local endpoints;
- explicit source trailing-slash intent;
- recursive regular file/directory copy;
- symbolic-link copy on supported hosts;
- `--delete` for extraneous destination entries;
- ordered inline include/exclude rules, merge filter files, and excluded-path
  delete protection including `--delete-excluded`;
- inherited per-directory merge rules with directory-relative anchoring;
- hard-link identity groups with `-H` through safe cross-platform handles;
- Windows read-only mapping under archive/permission preservation;
- mutation-free `--dry-run`;
- regular-file modification times at the exercised rsync/filesystem precision;
- rsync-style broad syntax and partial-transfer exit categories.

The Windows reference is independently differential-tested against WSL rsync
3.2.7 for trailing-slash contents, non-trailing directory naming with absent
and existing destinations, recursive binary/name handling, `--delete`,
`--delete-excluded`, `--dry-run`, ordered and inherited filters, merge files,
timestamps, hard-link identity, and multiple sources. The first differential run corrected an
author-test assumption about the absent-destination directory shape.

Not yet implemented:

- complete option and filter semantics;
- timestamps and the full metadata/capability mapping;
- rolling delta and interruption recovery;
- protocol-31 framing and remote-shell roles;
- daemon client/server, modules, and authentication;
- every remaining row in `COMPATIBILITY_MATRIX.csv`.

Passing these local unit tests is checkpoint evidence only. It is not Phase 0
completion and is not a benchmark success.
