# Phase 2 pattern and streaming contract

This is the Phase 2 authoring record. Version 1 incorporates these rules in
the agent-facing `TASK.md`.

## Pattern sources and syntax

- Without `-e` or `-f`, the first positional argument is the sole pattern.
  Otherwise all `-e` arguments and all lines of `-f` files are appended in
  encounter order. An empty pattern file contributes no patterns. A file is
  split at LF, including empty interior lines; a final unterminated line is a
  pattern, and a final LF does not add another pattern.
- Every nonempty pattern is an alternative. A zero-length pattern selects
  every logical line, including with `-w` or `-x`. With no patterns, no line
  is selected. Selection is inverted by `-v` only after the OR result.
- `-F` searches raw bytes. `-i` folds only ASCII A–Z. `-w` requires both
  adjacent bytes outside a matched span (when present) to be outside
  `[A-Za-z0-9_]`. `-x` requires the entire logical line to match.
- Regex mode accepts valid UTF-8 patterns and input. Its supported ERE subset
  is literals, `.`, bracket classes and ranges, `^`, `$`, grouping,
  alternation, and `?`, `*`, `+`, `{m,n}` repetition as parsed by the pinned
  Rust `regex` crate. ASCII case folding and ASCII word boundaries are used.
  Backreferences, `(?...)` extensions, locale classes such as
  `[[:alpha:]]`, and backslash escapes other than escaped ERE punctuation
  are usage errors (status 2). Regex `-x` tests a whole-line match; regex
  `-w` tests ASCII boundaries around a matched alternative.

## Logical lines and binary input

- LF ends a logical line and is omitted from matching. One immediately
  preceding CR is also omitted. A final unterminated line participates.
  Ordinary selected output always ends in LF.
- Fixed mode accepts arbitrary input bytes; regex mode rejects a logical
  line containing invalid UTF-8 with status 2. No replacement decoding is
  performed.
- A NUL switches normal output for that input to binary mode. Previously
  emitted lines remain visible. The first selected line after the switch
  emits one `Binary file <name> matches` notice and later selected lines
  emit no more text. `-c`, `-l`, `-L`, and `-q` continue to use their normal
  count/list/quiet results rather than the binary notice.
- Input is processed incrementally. At most the current logical line is
  retained by the scanner. Its content limit is exactly 16 MiB; LF and the
  optional immediately preceding CR do not count. A longer line is an input
  error (status 2). The scanner must handle arbitrary read boundaries.

The Phase 3/4 contracts record output-mode precedence, traversal, errors,
and platform path rules. Version 1 is published without a scored run yet.
