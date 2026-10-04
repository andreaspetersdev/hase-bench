# Phase 3 selection, traversal, and output contract

This authoring record extends `PHASE2_CONTRACT.md`. Version 1 includes these
rules in the agent-facing `TASK.md`.

## Selection and precedence

- A logical line is selected when any pattern matches, then `-v` inverts that
  decision. The resulting line count and process status use selected lines.
- Output modes have fixed precedence, regardless of option order:
  `-q` > `-l` > `-L` > `-c` > `-o` > ordinary lines. `-l` outputs each
  matching input name once; `-L` outputs each input name with no selected
  lines. `-c` outputs the number of selected lines per input, including zero.
  `-q` outputs nothing and stops at the first selected line.
- `-o` outputs non-overlapping, nonempty matched spans of selected lines.
  Choose the earliest next byte position; a tie chooses the pattern encountered
  first. Emit one LF-terminated record per span. A zero-width or empty-pattern
  match still selects the line but emits no span. `-v -o` selects inverted
  lines but emits no spans. Context options do not affect non-ordinary modes.
- `-A N`, `-B N`, and `-C N` configure following, preceding, and both
  context counts. Each accepts decimal 0–1000, attached or separate. A later
  option overwrites only the direction it sets. Context applies only to
  ordinary line output. Adjacent or overlapping groups share lines, and a
  single `--` line separates disjoint groups within one input. No group
  separator is inserted between different inputs.

## Presentation

- With multiple explicit inputs or recursion, filename prefixes are on by
  default; otherwise off. The last `-H` or `-h` overrides this. Stdin is
  named `(standard input)`. `-l` and `-L` always write input names.
- `-n` adds one-based logical line numbers. Prefix fields end in `:` for
  selected lines and `-` for context lines. `-o` repeats the filename and
  line number for each span. `-c` uses `name:` when filename prefixes are on.
- The Phase 2 binary transition applies to ordinary and `-o` output. Count,
  listing, and quiet modes retain their own output.

## Input order and path filters

- With no paths, read stdin. An explicit `-` reads stdin in its argv position.
  Explicit inputs retain argv order; each recursive directory visits entries
  in native component order. A directory without `-r` or `-R` is an error.
- `--include` and `--exclude` accept a following pattern or `=pattern`.
  Multiple patterns of each kind are ORed. `*` matches zero or more characters
  within one path component, `**` can cross `/`, and `?` matches one character
  within one component. All other characters are literal; matching is case
  sensitive. A pattern without `/` matches the basename. A pattern with `/`
  matches the slash-normalized path relative to the explicit root. Exclusion
  wins over inclusion, and an excluded directory is pruned. Without include
  patterns, all non-excluded files qualify. A non-UTF-8 path cannot match a
  textual filter; unfiltered fixed-mode traversal still processes it.
- `-r` does not follow symlinks or reparse links. `-R` follows file and
  directory links, suppressing repeated directory targets and loops. Unix
  uses device/inode identity; Windows uses canonical paths. Explicit directory roots share the visited
  set. Ordinary files are searched even if their contents are binary.

## Exit result

Status is 0 when the selected result exists, 1 when it does not, and 2 after
an input/usage/output error. Independent inputs continue after an error, and
already emitted selected output remains. `-q` stops at its first selected
line: an error already encountered retains status 2; later inputs are not
scanned or reported. `PHASE4_CONTRACT.md` records injected I/O and
broken-pipe behavior.
