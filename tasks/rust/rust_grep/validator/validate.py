#!/usr/bin/env python3
"""Authoritative black-box validator for the rust_grep executable."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


FIXTURES = (
    "phase1_smoke.py",
    "phase2_patterns.py",
    "phase2_streaming.py",
    "phase3_selection_output.py",
    "phase4_platform.py",
    "phase4_failures.py",
    "phase4_memory.py",
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    candidate = parser.parse_args().candidate.resolve(strict=True)
    if not candidate.is_file():
        parser.error("candidate must be a built executable")
    validator = Path(__file__).resolve().parent
    failures = 0
    for fixture in FIXTURES:
        try:
            result = subprocess.run(
                [sys.executable, str(validator / fixture), str(candidate)],
                capture_output=True,
                text=True,
                timeout=70 if fixture == "phase4_memory.py" else 25,
                check=False,
            )
        except subprocess.TimeoutExpired:
            print(f"FAIL {fixture}: timed out", flush=True)
            failures += 1
            continue
        print(f"{fixture}: {'PASS' if result.returncode == 0 else 'FAIL'}", flush=True)
        if result.stdout:
            print(result.stdout, end="" if result.stdout.endswith("\n") else "\n", flush=True)
        if result.stderr:
            print(result.stderr, file=sys.stderr, end="" if result.stderr.endswith("\n") else "\n", flush=True)
        failures += result.returncode != 0
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
