# RUST-GREP — cross-platform search utility

Implement the `rust_grep` command-line executable in this Rust 2024 project.
It must work natively on Windows and Linux. The starter compiles but leaves
major behavior unfinished. You may change the starter implementation and add
ordinary source/tests, but keep the `rust_grep` executable and its command-line
interface. Use the pinned `Cargo.lock`; `regex` 1.13.1 is available. Build and
run the visible tests with `cargo test --locked`.

This is a deliberately bounded grep-like tool, not full GNU grep. The rules
below are the version 1 contract; unsupported options or syntax must report
an error and exit 2 rather than silently changing meaning. Hidden tests
exercise the same contract independently of the visible tests.

## Command line and patterns

Support `-e`, `-f`, `-F`, `-E`, `-i`, `-v`, `-w`, `-x`, `-n`, `-H`, `-h`,
`-c`, `-l`, `-L`, `-q`, `-o`, `-A`, `-B`, `-C`, `-r`, `-R`, `--include`,
and `--exclude`. Short flags may be grouped. `-e`, `-f`, `-A`, `-B`, and
`-C` accept an attached or next argument. Also accept `--regexp=PATTERN`,
`--file=PATH`, and `--include`/`--exclude` with either `=PATTERN` or a
following argument. `--` ends option parsing. The last `-F`/`-E` and the
last `-H`/`-h` take effect. Unknown or malformed options exit 2.

Without `-e` or `-f`, the first positional argument is the sole pattern;
remaining operands are paths. Otherwise, `-e` patterns and lines from `-f`
files are appended in encounter order, and all positional operands are paths.
Split pattern files at LF: retain empty interior lines and a final
unterminated line, but do not add a pattern for a final LF. An empty pattern
file adds nothing. Patterns are OR alternatives. An empty pattern selects
every logical line, including with `-w` or `-x`; zero patterns select none.
Apply `-v` after the OR result.

`-F` matches raw bytes. `-i` folds ASCII A–Z only. `-w` requires bytes on
both sides of the matched span, when present, to be outside `[A-Za-z0-9_]`.
`-x` requires the whole logical line to match. Regex mode (the default or
`-E`) requires UTF-8 patterns and input. Its supported ERE subset is
literals, `.`, bracket classes and ranges, `^`, `$`, grouping, alternation,
and `?`, `*`, `+`, `{m,n}` repetition as parsed by the pinned Rust `regex`
crate. Use ASCII case folding and ASCII word boundaries. Reject
backreferences, `(?...)` extensions, locale classes such as `[[:alpha:]]`,
and backslash escapes other than escaped ERE punctuation with status 2.
Regex `-x` is a whole-line match; regex `-w` uses the same ASCII boundaries.

## Input and output

Read stdin if there are no paths; an explicit `-` reads stdin in its operand
position. Preserve explicit path order. A directory without recursion is an
error. `-r` recursively visits entries in native component order without
following directory links. `-R` follows file and directory links but
suppresses repeated directory targets and loops. On Unix, use device/inode
identity; on Windows, canonical target paths are acceptable for this version.
Explicit directory roots share the visited set.

`--include` and `--exclude` patterns are case-sensitive globs. `*` matches
within one component, `**` can cross `/`, and `?` matches one character
within one component; other characters are literal. A pattern without `/`
matches a basename. A pattern with `/` matches the slash-normalized path
relative to the explicit recursive root. Multiple patterns of each kind are
ORed; exclusion wins and excluded directories are pruned. Without includes,
all non-excluded files qualify. A non-UTF-8 path cannot match a textual
filter, but unfiltered fixed-mode traversal still processes it.

LF ends a logical line and is excluded from matching. One CR immediately
before LF is also excluded. Search a final unterminated line. Ordinary
selected output is LF-terminated. Fixed mode accepts arbitrary file bytes;
regex mode reports invalid UTF-8 on a logical line as an input error. A NUL
switches ordinary or `-o` output for that input to binary mode: keep any
earlier output, then emit `Binary file <name> matches` once on the first
selected line after the switch and no more selected-line text. `-c`, `-l`,
`-L`, and `-q` retain their own behavior. Process input incrementally.
The maximum logical-line content length is exactly 16 MiB; LF and its
immediately preceding CR do not count. A longer line exits 2.

Output-mode precedence is fixed, independent of option order:
`-q` > `-l` > `-L` > `-c` > `-o` > ordinary lines. `-q` writes nothing and
stops at the first selected line. `-l` writes each matching input name once;
`-L` writes each input name with no selected lines; `-c` writes selected-line
counts per input, including zero. `-o` emits non-overlapping, nonempty spans
of selected lines, one LF-terminated record per span. Choose the earliest
next byte position; ties use pattern encounter order. Empty/zero-width
matches still select a line but emit no span. `-v -o` emits no spans.

Filename prefixes are on by default for multiple explicit inputs or
recursion, otherwise off; the last `-H` or `-h` overrides this. Stdin is
named `(standard input)`. `-l` and `-L` always write names. `-n` adds
one-based logical line numbers. Selected-line prefix fields end in `:` and
context-line fields in `-`; `-o` repeats prefixes for each span. `-c` uses
`name:` when filename prefixes are on.

`-A N`, `-B N`, and `-C N` set following, preceding, and both context
counts. `N` is decimal 0–1000. A later option overwrites only the direction
it sets. Context applies only to ordinary output. Adjacent or overlapping
groups share lines; one `--` line separates disjoint groups within an input,
never across inputs.

## Paths, failures, and exit status

Write stdout as bytes. On Unix, fixed mode accepts non-UTF-8 native filenames
and writes their original bytes in filename prefixes. Regex mode rejects a
non-UTF-8 input path with a diagnostic and status 2. On Windows, accept
drive-qualified, UNC, and extended-length paths; retain the supplied spelling
in filename prefixes and encode redirected output as UTF-8. Console
code-page conversion is outside this task. Diagnostic path rendering may be
lossy.

Exit 0 if a selected result exists, 1 if none exists, and 2 on usage,
input, or output error. Continue independent inputs after an input error;
keep output already emitted. A quiet-mode match stops scanning: an earlier
error retains status 2, while later inputs are not touched. A read error
retains previously written output and exits 2. A broken stdout pipe exits 0
only after a selected result was established and no earlier error occurred;
otherwise it exits 2. Other output errors exit 2. Bound memory by the
current logical line, pattern state, context window, and output span state,
not by input-file size.
