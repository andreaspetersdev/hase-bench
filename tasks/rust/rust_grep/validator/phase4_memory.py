#!/usr/bin/env python3
"""Bounded-memory authoring probe over an input much larger than RAM use."""

from __future__ import annotations

import argparse
import ctypes
import os
import subprocess
import tempfile
import time
from pathlib import Path


def resident_bytes(process: subprocess.Popen[bytes]) -> int:
    if os.name != "nt":
        try:
            status = Path(f"/proc/{process.pid}/status").read_bytes()
        except FileNotFoundError:
            return 0
        for line in status.splitlines():
            if line.startswith(b"VmRSS:"):
                return int(line.split()[1]) * 1024
        return 0

    from ctypes import wintypes

    class Counters(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD), ("page_faults", wintypes.DWORD),
            ("peak_working_set", ctypes.c_size_t), ("working_set", ctypes.c_size_t),
            ("peak_paged_pool", ctypes.c_size_t), ("paged_pool", ctypes.c_size_t),
            ("peak_nonpaged_pool", ctypes.c_size_t), ("nonpaged_pool", ctypes.c_size_t),
            ("pagefile", ctypes.c_size_t), ("peak_pagefile", ctypes.c_size_t),
        ]

    counters = Counters()
    counters.cb = ctypes.sizeof(counters)
    query = ctypes.WinDLL("psapi", use_last_error=True).GetProcessMemoryInfo
    query.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
    query.restype = wintypes.BOOL
    if not query(process._handle, ctypes.byref(counters), counters.cb):
        return 0
    return counters.peak_working_set


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    program = parser.parse_args().candidate.resolve()
    with tempfile.TemporaryDirectory(prefix="rust-grep-memory-") as raw:
        source = Path(raw) / "large.txt"
        record = b"x" * (96 * 1024 - 1) + b"\n"
        with source.open("wb") as stream:
            for _ in range(1024):
                stream.write(record)
        process = subprocess.Popen((str(program), "-F", "-B16", "needle", str(source)),
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        peak = 0
        deadline = time.monotonic() + 60
        while process.poll() is None:
            peak = max(peak, resident_bytes(process))
            if time.monotonic() > deadline:
                process.kill()
                raise TimeoutError("96 MiB scan exceeded 60 seconds")
            time.sleep(0.01)
        peak = max(peak, resident_bytes(process))
        stdout, stderr = process.communicate()
        if (process.returncode, stdout) != (1, b""):
            raise AssertionError(f"scan failed: {process.returncode}, {stdout[:100]!r}, {stderr[:100]!r}")
        limit = 64 * 1024 * 1024
        if peak == 0 or peak > limit:
            raise AssertionError(f"peak RSS {peak / 1024 / 1024:.1f} MiB exceeds "
                                 f"the 64 MiB limit or was not measurable")
        print(f"PASS bounded_memory: 96 MiB file, peak RSS {peak / 1024 / 1024:.1f} MiB")


if __name__ == "__main__":
    main()
