# Author reference status

This is the in-progress author-only reference, not a published solution. It
depends on the starter crate for the reviewed public parsing and domain types.

Implemented and tested:

- local endpoints;
- explicit source trailing-slash intent;
- recursive regular file/directory copy;
- symbolic-link copy on supported hosts;
- `--delete` for extraneous destination entries;
- mutation-free `--dry-run`;
- rsync-style broad syntax and partial-transfer exit categories.

Not yet implemented:

- complete option and filter semantics;
- timestamps and the full metadata/capability mapping;
- rolling delta and interruption recovery;
- protocol-31 framing and remote-shell roles;
- daemon client/server, modules, and authentication;
- every remaining row in `COMPATIBILITY_MATRIX.csv`.

Passing these local unit tests is checkpoint evidence only. It is not Phase 0
completion and is not a benchmark success.
