#!/usr/bin/env python3
"""Authoring smoke fixture, not the independent benchmark validator."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def execute(program: str, *args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run((program, *args), capture_output=True, check=False, timeout=15)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    grep = shutil.which("grep")
    if grep is None:
        parser.error("GNU grep is required")
    version = execute(grep, "--version")
    if version.returncode or b"GNU grep" not in version.stdout:
        parser.error("the grep on PATH is not GNU grep")
    print(version.stdout.splitlines()[0].decode("ascii", "replace"))

    with tempfile.TemporaryDirectory(prefix="rust-grep-phase1-") as raw:
        root = Path(raw)
        source = root / "sample.txt"
        source.write_bytes(b"alpha\nbeta alpha\nlast")
        cases = [
            ("fixed", ("-F", "alpha", str(source))),
            ("regex", ("-E", "a(lpha|st)$", str(source))),
            ("count", ("-c", "missing", str(source))),
            ("number", ("-n", "^beta", str(source))),
        ]
        for name, arguments in cases:
            expected = execute(grep, *arguments)
            observed = execute(str(args.candidate), *arguments)
            # The Windows GNU port uses CRLF for redirected stdout.
            normalized = expected.stdout.replace(b"\r\n", b"\n") if os.name == "nt" else expected.stdout
            if (observed.returncode, observed.stdout) != (expected.returncode, normalized):
                print(f"FAIL {name}: oracle=({expected.returncode}, {normalized!r}), "
                      f"candidate=({observed.returncode}, {observed.stdout!r})")
                return 1
            print(f"PASS {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
