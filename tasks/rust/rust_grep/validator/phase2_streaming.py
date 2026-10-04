#!/usr/bin/env python3
"""Independent Phase 2 byte-stream fixture for an already-built candidate."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


def check(program: Path, name: str, args: list[str], data: bytes,
          status: int, stdout: bytes, error_fragment: bytes | None = None) -> None:
    result = subprocess.run((str(program), *args), input=data,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False, timeout=15)
    if (result.returncode, result.stdout) != (status, stdout):
        raise AssertionError(f"{name}: got {result.returncode}, "
                             f"stdout={result.stdout[:100]!r}, stderr={result.stderr[:100]!r}")
    if error_fragment is not None and error_fragment not in result.stderr:
        raise AssertionError(f"{name}: missing {error_fragment!r} in {result.stderr!r}")
    print(f"PASS {name}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    candidate = parser.parse_args().candidate.resolve()
    check(candidate, "crlf_and_final_line", ["-F", "needle"],
          b"needle\r\nother\nneedle", 0, b"needle\nneedle\n")
    check(candidate, "chunk_boundary_match", ["-F", "-q", "needle"],
          b"x" * 8191 + b"needle\n", 0, b"")
    check(candidate, "fixed_invalid_utf8", ["-F", "\u00ff"],
          b"\xff\n", 1, b"")
    check(candidate, "regex_invalid_utf8", ["."],
          b"\xff\n", 2, b"", b"not UTF-8")
    check(candidate, "nul_binary_output", ["-F", "needle"],
          b"\x00needle\n", 0, b"Binary file (standard input) matches\n")
    check(candidate, "nul_count_output", ["-F", "-c", "needle"],
          b"\x00needle\nneedle\n", 0, b"2\n")
    check(candidate, "nul_quiet_output", ["-F", "-q", "needle"],
          b"\x00needle\n", 0, b"")
    check(candidate, "late_nul_switches_normal_output", ["-F", "needle"],
          b"needle\n\x00needle\nneedle\n", 0,
          b"needle\nBinary file (standard input) matches\n")
    maximum = 16 * 1024 * 1024
    check(candidate, "maximum_line", ["-F", "-q", "a"],
          b"a" * maximum, 0, b"")
    check(candidate, "maximum_crlf_line", ["-F", "-q", "a"],
          b"a" * maximum + b"\r\n", 0, b"")
    check(candidate, "oversize_line", ["-F", "-q", "a"],
          b"a" * (maximum + 1), 2, b"", b"exceeds 16 MiB")


if __name__ == "__main__":
    main()
