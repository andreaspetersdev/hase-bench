#!/usr/bin/env python3
"""Independent Phase 2 pattern fixture for an already-built candidate binary."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def run(program: Path, args: list[str], data: bytes) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run((str(program), *args), input=data,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False, timeout=15)


def check(program: Path, name: str, args: list[str], data: bytes,
          status: int, stdout: bytes, error_fragment: bytes | None = None) -> None:
    actual = run(program, args, data)
    if actual.returncode != status or actual.stdout != stdout:
        raise AssertionError(f"{name}: expected {(status, stdout)!r}, got "
                             f"{(actual.returncode, actual.stdout, actual.stderr)!r}")
    if error_fragment is not None and error_fragment not in actual.stderr:
        raise AssertionError(f"{name}: missing {error_fragment!r} in {actual.stderr!r}")
    print(f"PASS {name}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    candidate = parser.parse_args().candidate.resolve()
    grep = shutil.which("grep")
    if grep is None:
        parser.error("GNU grep is required for Phase 2 differential checks")
    version = subprocess.run((grep, "--version"), capture_output=True, check=False)
    if version.returncode or b"GNU grep" not in version.stdout:
        parser.error("grep on PATH is not GNU grep")
    print(version.stdout.splitlines()[0].decode("ascii", "replace"))
    sample = b"cat\nscatter\ncat-\nalpha\n\n"
    check(candidate, "pattern_order_and_or", ["-F", "-e", "missing", "-e", "cat"],
          sample, 0, b"cat\nscatter\ncat-\n")
    check(candidate, "empty_literal_selects_all", ["-F", "-e", ""],
          sample, 0, sample)
    check(candidate, "empty_regex_selects_all", ["-e", ""], sample, 0, sample)
    check(candidate, "ascii_case_only", ["-F", "-i", "A"],
          b"a\nA\n\xc3\xa1\n", 0, b"a\nA\n")
    check(candidate, "ascii_word_boundaries", ["-F", "-w", "cat"],
          sample, 0, b"cat\ncat-\n")
    check(candidate, "whole_line_fixed", ["-F", "-x", "cat"],
          sample, 0, b"cat\n")
    check(candidate, "whole_line_ere_alternative", ["-x", "a|ab"],
          b"ab\na\n", 0, b"ab\na\n")
    check(candidate, "word_ere_alternative", ["-w", "a|ab"],
          b"ab\nabc\n", 0, b"ab\n")
    for name, expression in (("escape_class", r"\d"),
                             ("backreference", r"(a)\1"),
                             ("inline_flag", "(?i)a"),
                             ("locale_class", "[[:alpha:]]"),
                             ("nested_locale_class", "[a[:alpha:]]")):
        check(candidate, name, [expression], b"a\n1\n", 2, b"", b"unsupported")
    check(candidate, "literal_paren_question_class", ["[(?]"],
          b"(\n?\n", 0, b"(\n?\n")
    with tempfile.TemporaryDirectory(prefix="rust-grep-patterns-") as raw:
        source = Path(raw) / "patterns"
        source.write_bytes(b"missing\ncat")
        check(candidate, "unterminated_pattern_file", ["-F", "-f", str(source)],
              sample, 0, b"cat\nscatter\ncat-\n")
        source.write_bytes(b"\n")
        check(candidate, "empty_pattern_file_line", ["-F", "-f", str(source)],
              sample, 0, sample)
        source.write_bytes(b"")
        check(candidate, "empty_pattern_file", ["-F", "-f", str(source)],
              sample, 1, b"")
        source.write_bytes(b"\xff\n")
        check(candidate, "raw_byte_fixed_pattern", ["-F", "-f", str(source)],
              b"plain\n\xff\n", 0, b"\xff\n")
        check(candidate, "non_utf8_regex_pattern", ["-f", str(source)],
              b"plain\n", 2, b"", b"not UTF-8")
    for name, arguments, data in (
        ("fixed_or", ["-F", "-e", "cat", "-e", "alpha"], sample),
        ("ere_whole", ["-E", "-x", "a|ab"], b"ab\na\nabc\n"),
        ("ere_word", ["-E", "-w", "a|ab"], b"ab\nabc\n"),
    ):
        environment = dict(os.environ, LC_ALL="C")
        oracle = subprocess.run((grep, *arguments), input=data, capture_output=True,
                                check=False, env=environment, timeout=15)
        actual = run(candidate, arguments, data)
        oracle_output = (oracle.stdout.replace(b"\r\n", b"\n")
                         if os.name == "nt" else oracle.stdout)
        if (actual.returncode, actual.stdout) != (oracle.returncode, oracle_output):
            raise AssertionError(f"GNU differential {name}: "
                                 f"{(oracle.returncode, oracle_output)!r} vs "
                                 f"{(actual.returncode, actual.stdout)!r}")
        print(f"PASS GNU differential {name}")


if __name__ == "__main__":
    main()
