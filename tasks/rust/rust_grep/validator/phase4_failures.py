#!/usr/bin/env python3
"""External process and output-failure checks for the author reference."""

from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path


def broken_stdout(program: Path, arguments: list[str], expected: int) -> None:
    process = subprocess.Popen((str(program), *arguments), stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE)
    assert process.stdout is not None
    process.stdout.close()
    process.stdout = None
    assert process.stderr is not None
    try:
        _, stderr = process.communicate(timeout=15)
    except subprocess.TimeoutExpired:
        process.kill()
        process.communicate()
        raise
    status = process.returncode
    if status != expected:
        raise AssertionError(f"broken stdout {arguments}: status={status}, "
                             f"expected={expected}, stderr={stderr!r}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    program = parser.parse_args().candidate.resolve()
    with tempfile.TemporaryDirectory(prefix="rust-grep-failures-") as raw:
        root = Path(raw)
        matching = root / "match.txt"
        absent = root / "absent.txt"
        # The long first line gives the parent time to close the pipe before
        # the child reaches its first output write.
        matching.write_bytes(b"x" * (1024 * 1024) + b"hit\n")
        absent.write_bytes(b"x" * (1024 * 1024) + b"\n")
        broken_stdout(program, ["-F", "hit", str(matching)], 0)
        print("PASS broken_pipe_after_match")
        broken_stdout(program, ["-F", "-c", "hit", str(absent)], 2)
        print("PASS broken_pipe_without_match")


if __name__ == "__main__":
    main()
