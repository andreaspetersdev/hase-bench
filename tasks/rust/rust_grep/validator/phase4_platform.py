#!/usr/bin/env python3
"""Native Windows/Linux path and access fixtures for the author reference."""

from __future__ import annotations

import argparse
import ctypes
import os
import subprocess
import tempfile
from pathlib import Path


def run(program: Path, *arguments: str | bytes) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run((str(program), *arguments), capture_output=True, check=False, timeout=15)


def expect(name: str, result: subprocess.CompletedProcess[bytes],
           status: int, output: bytes, error: bytes | None = None) -> None:
    if (result.returncode, result.stdout) != (status, output):
        raise AssertionError(f"{name}: got "
                             f"{(result.returncode, result.stdout, result.stderr)!r}")
    if error is not None and error not in result.stderr:
        raise AssertionError(f"{name}: missing {error!r} in {result.stderr!r}")
    print(f"PASS {name}")


def windows_checks(program: Path, root: Path) -> None:
    mixed = root / "MiXeD.txt"
    mixed.write_bytes(b"hit\r\n")
    expect("windows_drive_and_spelling", run(program, "-F", "-H", "hit", str(mixed)),
           0, str(mixed).encode("utf-8") + b":hit\n")
    extended = "\\\\?\\" + str(mixed)
    expect("windows_extended_path", run(program, "-F", "-H", "hit", extended),
           0, extended.encode("utf-8") + b":hit\n")

    fixture = os.environ.get("RUST_GREP_UNC_FIXTURE")
    if fixture and fixture.startswith("\\\\"):
        result = run(program, "-F", "-H", "hit", fixture)
        if result.returncode not in (0, 1):
            raise AssertionError(f"UNC fixture unreadable: {result.stderr!r}")
        print("PASS windows_unc_fixture")
    else:
        print("SKIP windows_unc_fixture (RUST_GREP_UNC_FIXTURE not configured)")

    from ctypes import wintypes

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel.CreateFileW
    create_file.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                            ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD,
                            wintypes.HANDLE]
    create_file.restype = wintypes.HANDLE
    close_handle = kernel.CloseHandle
    close_handle.argtypes = [wintypes.HANDLE]
    close_handle.restype = wintypes.BOOL
    handle = create_file(str(mixed), 0x80000000, 0, None, 3, 0x80, None)
    invalid = ctypes.c_void_p(-1).value
    if handle == invalid:
        raise OSError(ctypes.get_last_error(), "exclusive CreateFileW failed")
    try:
        expect("windows_share_denial", run(program, "-F", "hit", str(mixed)),
               2, b"", b"MiXeD.txt")
    finally:
        close_handle(handle)


def linux_checks(program: Path, root: Path) -> None:
    raw = os.fsencode(root)
    non_utf8 = raw + b"/raw-\xff.txt"
    descriptor = os.open(non_utf8, os.O_CREAT | os.O_WRONLY | os.O_TRUNC, 0o644)
    try:
        os.write(descriptor, b"hit\n")
    finally:
        os.close(descriptor)
    expect("linux_raw_filename_output", run(program, "-F", "-H", "hit", non_utf8),
           0, non_utf8 + b":hit\n")
    expect("linux_raw_filename_recursive", run(program, "-F", "-r", "hit", str(root)),
           0, non_utf8 + b":hit\n")
    expect("linux_regex_filename_error", run(program, "-H", "hit", non_utf8),
           2, b"", b"regex path is not UTF-8")

    locked = root / "unreadable.txt"
    locked.write_bytes(b"hit\n")
    locked.chmod(0)
    try:
        if os.geteuid() == 0:
            print("SKIP linux_unreadable_file (runner is root)")
        else:
            expect("linux_unreadable_file", run(program, "-F", "hit", str(locked)),
                   2, b"", b"unreadable.txt")
    finally:
        locked.chmod(0o600)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    program = parser.parse_args().candidate.resolve()
    with tempfile.TemporaryDirectory(prefix="rust-grep-platform-") as raw:
        root = Path(raw)
        if os.name == "nt":
            windows_checks(program, root)
        else:
            linux_checks(program, root)


if __name__ == "__main__":
    main()
