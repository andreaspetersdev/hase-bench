# RUST-GREP version 1 behavior matrix

This is the implementation and validation map for version 1. `V` means a
visible test, `H` an independent hidden test, and `D` a GNU-grep differential
fixture. The external hidden suite is `validator/validate.py`.

| Area / pinned behavior | Planned module boundary | Evidence | Platform/capability rule |
| --- | --- | --- | --- |
| CLI grammar: options end at `--`; `-e` and `-f` append patterns in encounter order; malformed/unsupported options exit 2 | `cli` | V, H | identical byte arguments after platform argument decoding |
| Pattern sources: one positional pattern when no `-e`/`-f`; empty patterns select every logical line; pattern files split on LF and retain a final unterminated pattern | `patterns` | V, H, D | pattern files must be valid UTF-8 in regex mode |
| Fixed matching: raw-byte substring; `-i` folds ASCII A–Z only; `-w` checks ASCII word boundaries; `-x` requires whole logical line | `matcher::fixed` | V, H, D | binary-safe on both systems |
| Regex matching: portable ERE subset; unsupported constructs have an exit-2 diagnostic; regex `-i`, `-w`, and `-x` retain the pinned ASCII/line rules | `matcher::regex` | V, H, D | UTF-8-only; no locale dependence |
| Logical lines: LF terminates; CR immediately before LF is not part of line content; final unterminated line is searchable; emitted text uses contract LF terminators | `stream` | V, H, D | CRLF and LF fixtures on both systems; Windows oracle comparisons normalize its observed CRLF output |
| Binary / invalid UTF-8: fixed mode searches bytes; a NUL makes normal output `binary file matches`; `-c`, `-l`, `-L`, and `-q` retain their mode semantics; regex invalid UTF-8 is an error | `stream`, `output` | V, H | no implicit lossy decoding |
| Selection and modes: OR patterns, then optional `-v`; `-c`, `-l`, `-L`, `-q`, and `-o` are mutually resolved by documented precedence | `selection`, `output` | V, H, D | broken pipe is success only after a match result is established |
| Presentation: deterministic filename/line prefixes, `-H`/`-h`, `-n`, `-o` span order, `-A/-B/-C`, and a single `--` group separator | `output` | V, H, D | stdout is bytes; Windows console conversion is outside redirected-output validation |
| Inputs: stdin for no paths or `-`; explicit files retain argv order; a directory without recursion is error | `inputs` | V, H, D | native path representation; diagnostics use display paths |
| Recursive traversal: `-r` does not follow directory links; `-R` follows them with file-identity loop detection; entries are sorted by platform-native path components; include/exclude use slash-normalized relative paths | `walk` | V, H | Windows reparse points and Linux symlinks have separate capability fixtures |
| Error precedence: continue independent inputs, print one diagnostic per failed input, return 2 if no quiet match occurred first; any selected result remains written | `run` | V, H | permissions and share-access cases are capability-gated |
| Resource limits: process lines incrementally; retain at most current line, patterns, selected context window, and output span state; maximum logical line is 16 MiB and larger input exits 2 | `stream` | H | memory-limit fixture and 16 MiB boundary run on both systems |

The fixture map names implemented `V` and `H` obligations. Differential
cases are independent of the candidate matcher and record GNU grep's version.

| Area | Visible fixture | Independent fixture | Status |
| --- | --- | --- | --- |
| CLI grammar | `starter/tests/visible.rs::cli_pattern_sources` | `validator/phase3_selection_output.py::{option_terminator,unsupported_option,missing_context_argument,invalid_context}` | Phase 3 implemented on both OS families |
| Pattern sources | `starter/tests/visible.rs::patterns_from_arguments_and_files` | `validator/phase2_patterns.py::{pattern_order_and_or,empty_literal_selects_all,empty_regex_selects_all,unterminated_pattern_file,empty_pattern_file_line,empty_pattern_file}` | Phase 2 implemented on both OS families |
| Fixed matching | `starter/tests/visible.rs::fixed_ascii_boundaries` | `validator/phase2_patterns.py::{ascii_case_only,ascii_word_boundaries,whole_line_fixed,raw_byte_fixed_pattern}` | Phase 2 implemented on both OS families |
| Regex matching | `starter/tests/visible.rs::ere_match_and_line_number` | `validator/phase2_patterns.py::{whole_line_ere_alternative,word_ere_alternative,escape_class,backreference,inline_flag,locale_class,non_utf8_regex_pattern}` | Phase 2 implemented on both OS families |
| Logical lines | `starter/tests/visible.rs::fixed_match_preserves_bytes_and_uses_lf` | `validator/phase2_streaming.py::{crlf_and_final_line,chunk_boundary_match,maximum_line,maximum_crlf_line,oversize_line}` | Phase 2 implemented on both OS families |
| Binary and UTF-8 | `starter/tests/visible.rs::binary_modes` | `validator/phase2_streaming.py::{fixed_invalid_utf8,regex_invalid_utf8,nul_binary_output,nul_count_output,nul_quiet_output,late_nul_switches_normal_output}` | Phase 2 implemented on both OS families |
| Selection modes | `starter/tests/visible.rs::selection_modes` | `validator/phase3_selection_output.py::{count_inverted,quiet_wins_modes,files_with_wins_count,files_without,files_without_matching,files_with_beats_files_without,count_beats_only}` | Phase 3 implemented on both OS families |
| Presentation | `starter/tests/visible.rs::presentation` | `validator/phase3_selection_output.py::{only_fixed_nonoverlap,only_multiple_patterns,only_regex_word,inverted_only_has_no_spans,context_groups,context_overlap,only_prefixes_each_span,context_filename_prefixes}` | Phase 3 implemented on both OS families |
| Inputs | `starter/tests/visible.rs::{cli_pattern_sources,recursive_tree}` | `validator/phase3_selection_output.py::{explicit_input_order,force_filename,suppress_filename,count_per_file,files_with_paths}` | Phase 3 implemented on both OS families |
| Traversal | `starter/tests/visible.rs::recursive_tree` | `validator/phase3_selection_output.py::{recursive_include_exclude,exclude_wins_include,double_star_path_filter,recursive_no_follow_link,recursive_follow_loop_once}` | Phase 3 implemented; link tests pass on WSL, Windows capability unavailable |
| Error precedence | `starter/tests/visible.rs::exit_status` | `validator/phase3_selection_output.py::{quiet_before_later_error,quiet_after_prior_error,selected_output_survives_error}`, `validator/phase4_failures.py`, and injected reader/writer author-reference unit tests | Black-box failures pass on both OS families; portable post-line read injection is reference-only evidence |
| Resource limits | `starter/tests/visible.rs::long_line` | `validator/phase2_streaming.py::{maximum_line,maximum_crlf_line,oversize_line}` and `validator/phase4_memory.py` | Line bound and 96 MiB file / 64 MiB peak-RSS limit pass on both OS families |
| Native paths and access | task contract | `validator/phase4_platform.py` | Linux non-UTF-8 and permission cases pass; Windows drive, extended path, and share denial pass; UNC and directory-link cases are capability-skipped |

Phase 1 evidence is in the starter/reference visible tests and
`validator/phase1_smoke.py`. The current suite has 12 visible tests and a
one-byte-buffer unit test. Phase 2 has 33 independent pattern/streaming cases
on each platform, including three GNU grep 3.11 comparisons. Phase 3 adds
34 passing selection/output/traversal cases on Windows and 36 on Ubuntu WSL,
including four more GNU grep comparisons; the two link tests are capability
skips on Windows. Phase 4 adds native-path, access, broken-pipe,
injected-reader, and bounded-memory evidence on both systems. Phase 5
integrates black-box fixtures into the authoritative hidden validator and
publishes version 1. The Windows UNC and directory-link exceptions remain
capability skips and must be recorded with scored runs.
