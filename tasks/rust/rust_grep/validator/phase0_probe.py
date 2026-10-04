#!/usr/bin/env python3
"""Portable Phase 0 oracle/capability smoke check; not a benchmark validator."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path


def run(*args: str, stdin: bytes | None = None) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(args, input=stdin, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=False)


def main() -> int:
    grep = shutil.which("grep")
    if grep is None:
        print("FAIL: GNU grep is not on PATH")
        return 1
    version = run(grep, "--version")
    if version.returncode != 0 or b"GNU grep" not in version.stdout:
        print("FAIL: PATH grep is not GNU grep")
        return 1
    print("platform=" + platform.platform())
    print("grep=" + version.stdout.splitlines()[0].decode("ascii", "replace"))
    with tempfile.TemporaryDirectory(prefix="rust-grep-phase0-") as raw:
        root = Path(raw)
        sample = root / "sample.txt"
        sample.write_bytes(b"alpha\nbeta alpha\nlast")
        checks = [
            (("-F", "alpha", str(sample)), 0, b"alpha\nbeta alpha\n"),
            (("-n", "^beta", str(sample)), 0, b"2:beta alpha\n"),
            (("-c", "missing", str(sample)), 1, b"0\n"),
            (("-E", "a(lpha|st)$", str(sample)), 0,
             b"alpha\nbeta alpha\nlast\n"),
        ]
        for arguments, expected_status, expected_stdout in checks:
            result = run(grep, *arguments)
            oracle_stdout = (expected_stdout.replace(b"\n", b"\r\n")
                             if os.name == "nt" else expected_stdout)
            if result.returncode != expected_status or result.stdout != oracle_stdout:
                print(f"FAIL: oracle mismatch for {arguments!r}: "
                      f"status={result.returncode}, stdout={result.stdout!r}")
                return 1
        crlf = root / "crlf.txt"
        crlf.write_bytes(b"alpha\r\nbeta\r\n")
        crlf_result = run(grep, "-F", "alpha", str(crlf))
        if crlf_result.returncode != 0:
            print("FAIL: CRLF GNU grep smoke test failed")
            return 1
        print("crlf_output=" + repr(crlf_result.stdout))
        nested = root / "tree" / "nested"
        nested.mkdir(parents=True)
        (root / "tree" / "z.txt").write_text("needle\n", encoding="utf-8")
        (nested / "a.txt").write_text("needle\n", encoding="utf-8")
        recursive = run(grep, "-r", "-l", "needle", str(root / "tree"))
        if recursive.returncode != 0:
            print("FAIL: recursive GNU grep smoke test failed")
            return 1
        print("recursive_paths=" + repr(recursive.stdout.splitlines()))
        print("cwd_bytes=" + repr(os.fsencode(root)))
    print("PASS: GNU grep portable-subset smoke fixture")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
