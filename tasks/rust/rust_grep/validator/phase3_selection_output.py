#!/usr/bin/env python3
"""Independent Phase 3 selection, traversal, and output fixtures."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def check(program: Path, name: str, args: list[str], data: bytes | None,
          status: int, stdout: bytes, stderr_contains: bytes | None = None) -> None:
    result = subprocess.run((str(program), *args), input=data,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False, timeout=15)
    if (result.returncode, result.stdout) != (status, stdout):
        raise AssertionError(f"{name}: expected {(status, stdout)!r}, got "
                             f"{(result.returncode, result.stdout, result.stderr)!r}")
    if stderr_contains is not None and stderr_contains not in result.stderr:
        raise AssertionError(f"{name}: missing {stderr_contains!r} in {result.stderr!r}")
    print(f"PASS {name}")


def display(path: Path) -> bytes:
    return str(path).encode("utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    program = parser.parse_args().candidate.resolve()
    grep = shutil.which("grep")
    if grep is None:
        parser.error("GNU grep is required for Phase 3 differential checks")
    version = subprocess.run((grep, "--version"), capture_output=True, check=False)
    if version.returncode or b"GNU grep" not in version.stdout:
        parser.error("grep on PATH is not GNU grep")
    print(version.stdout.splitlines()[0].decode("ascii", "replace"))
    check(program, "option_terminator", ["-F", "--", "-dash"],
          b"-dash\n", 0, b"-dash\n")
    check(program, "unsupported_option", ["-Z", "hit"],
          b"hit\n", 2, b"", b"unsupported option")
    check(program, "missing_context_argument", ["-A"],
          b"hit\n", 2, b"", b"requires an argument")
    check(program, "only_fixed_nonoverlap", ["-F", "-o", "ana"],
          b"banana\n", 0, b"ana\n")
    check(program, "only_multiple_patterns", ["-F", "-o", "-e", "ana", "-e", "ban"],
          b"banana\n", 0, b"ban\nana\n")
    check(program, "only_regex_word", ["-o", "-w", "a|ab"],
          b"ab abc\n", 0, b"ab\n")
    check(program, "inverted_only_has_no_spans", ["-v", "-o", "missing"],
          b"alpha\n", 0, b"")
    check(program, "count_inverted", ["-F", "-v", "-c", "hit"],
          b"hit\nmiss\nmiss\n", 0, b"2\n")
    check(program, "quiet_wins_modes", ["-F", "-q", "-c", "-l", "hit"],
          b"hit\n", 0, b"")
    check(program, "files_with_wins_count", ["-F", "-l", "-c", "hit"],
          b"hit\n", 0, b"(standard input)\n")
    check(program, "files_without", ["-F", "-L", "hit"],
          b"miss\n", 0, b"(standard input)\n")
    check(program, "files_without_matching", ["-F", "-L", "hit"],
          b"hit\n", 1, b"")
    check(program, "files_with_beats_files_without", ["-F", "-L", "-l", "hit"],
          b"hit\n", 0, b"(standard input)\n")
    check(program, "count_beats_only", ["-F", "-o", "-c", "hit"],
          b"hit hit\n", 0, b"1\n")
    check(program, "context_groups", ["-F", "-n", "-B1", "-A1", "hit"],
          b"a\nb\nhit1\nc\nd\ne\nf\nhit2\ng\n", 0,
          b"2-b\n3:hit1\n4-c\n--\n7-f\n8:hit2\n9-g\n")
    check(program, "context_overlap", ["-F", "-C1", "hit"],
          b"a\nhit\nhit\nz\n", 0, b"a\nhit\nhit\nz\n")
    check(program, "invalid_context", ["-A1001", "hit"],
          b"hit\n", 2, b"", b"context count")
    with tempfile.TemporaryDirectory(prefix="rust-grep-phase3-") as raw:
        root = Path(raw)
        first = root / "a.txt"
        second = root / "b.log"
        first.write_bytes(b"hit\nmiss\n")
        second.write_bytes(b"miss\nhit\n")
        check(program, "explicit_input_order", ["-F", "hit", str(second), str(first)], None,
              0, display(second) + b":hit\n" + display(first) + b":hit\n")
        check(program, "force_filename", ["-F", "-H", "hit", str(first)], None,
              0, display(first) + b":hit\n")
        check(program, "suppress_filename", ["-F", "-h", "hit", str(first), str(second)], None,
              0, b"hit\nhit\n")
        check(program, "count_per_file", ["-F", "-c", "hit", str(first), str(second)], None,
              0, display(first) + b":1\n" + display(second) + b":1\n")
        check(program, "only_prefixes_each_span", ["-F", "-H", "-n", "-o", "i", str(first)], None,
              0, display(first) + b":1:i\n" + display(first) + b":2:i\n")
        check(program, "context_filename_prefixes", ["-F", "-H", "-n", "-B1", "-A1", "hit", str(first)], None,
              0, display(first) + b":1:hit\n" + display(first) + b"-2-miss\n")
        check(program, "files_with_paths", ["-F", "-l", "hit", str(first), str(second)], None,
              0, display(first) + b"\n" + display(second) + b"\n")
        check(program, "quiet_before_later_error", ["-F", "-q", "hit", str(first), str(root / "missing")], None,
              0, b"")
        check(program, "quiet_after_prior_error", ["-F", "-q", "hit", str(root / "missing"), str(first)], None,
              2, b"", b"missing")
        check(program, "selected_output_survives_error", ["-F", "hit", str(first), str(root / "missing")], None,
              2, display(first) + b":hit\n", b"missing")

        sub = root / "sub"
        skipped = root / "skip"
        sub.mkdir()
        skipped.mkdir()
        nested = sub / "c.txt"
        nested.write_bytes(b"hit\n")
        (skipped / "d.txt").write_bytes(b"hit\n")
        check(program, "recursive_include_exclude", ["-F", "-r", "--include=*.txt",
              "--exclude=skip", "hit", str(root)], None, 0,
              display(first) + b":hit\n" + display(nested) + b":hit\n")
        check(program, "exclude_wins_include", ["-F", "-r", "--include=*.txt",
              "--exclude=sub/c.txt", "--exclude=skip", "hit", str(root)], None, 0,
              display(first) + b":hit\n")
        check(program, "double_star_path_filter", ["-F", "-r", "--include", "sub/**",
              "hit", str(root)], None, 0, display(nested) + b":hit\n")
        loop = sub / "z_loop"
        try:
            os.symlink(root, loop, target_is_directory=True)
        except OSError:
            print("SKIP symlink_loop (directory symlink capability unavailable)")
        else:
            check(program, "recursive_no_follow_link", ["-F", "-r", "--include=*.txt",
                  "--exclude=skip", "hit", str(root)], None, 0,
                  display(first) + b":hit\n" + display(nested) + b":hit\n")
            check(program, "recursive_follow_loop_once", ["-F", "-R", "--include=*.txt",
                  "--exclude=skip", "hit", str(root)], None, 0,
                  display(first) + b":hit\n" + display(nested) + b":hit\n")

    for name, arguments, data in (
        ("only", ["-F", "-o", "ana"], b"banana\n"),
        ("count", ["-F", "-c", "hit"], b"hit\nmiss\n"),
        ("invert", ["-F", "-v", "hit"], b"hit\nmiss\n"),
        ("context", ["-F", "-n", "-B1", "-A1", "hit"],
         b"a\nhit\nb\nc\nd\nhit\ne\n"),
    ):
        oracle = subprocess.run((grep, *arguments), input=data, capture_output=True,
                                check=False, env=dict(os.environ, LC_ALL="C"), timeout=15)
        actual = subprocess.run((str(program), *arguments), input=data,
                                capture_output=True, check=False, timeout=15)
        expected = (oracle.stdout.replace(b"\r\n", b"\n")
                    if os.name == "nt" else oracle.stdout)
        if (actual.returncode, actual.stdout) != (oracle.returncode, expected):
            raise AssertionError(f"GNU differential {name}: "
                                 f"{(oracle.returncode, expected)!r} vs "
                                 f"{(actual.returncode, actual.stdout)!r}")
        print(f"PASS GNU differential {name}")


if __name__ == "__main__":
    main()
