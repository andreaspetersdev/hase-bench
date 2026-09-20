#!/usr/bin/env python3
"""Probe Windows filesystem capabilities used by RUST-RSYNC validation."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import tempfile


def attempt(action) -> tuple[bool, str | None]:
    try:
        action()
        return True, None
    except OSError as error:
        return False, f"{type(error).__name__}: {error}"


def command_capability(arguments: list[str]) -> tuple[bool, str | None]:
    executable = shutil.which(arguments[0])
    if executable is None:
        return False, f"executable not found: {arguments[0]}"
    try:
        result = subprocess.run(
            [executable, *arguments[1:]],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=10.0,
        )
    except (OSError, subprocess.SubprocessError) as error:
        return False, f"{type(error).__name__}: {error}"
    if result.returncode != 0:
        output = result.stdout.strip()
        return False, f"exit {result.returncode}: {output}"
    return True, None


def probe_readonly_mapping(path: Path) -> tuple[bool, str | None]:
    if os.name != "nt":
        return False, "Windows-only FILE_ATTRIBUTE_READONLY probe"
    try:
        path.write_bytes(b"permissions")
        os.chmod(path, stat.S_IREAD)
        attributes = path.stat().st_file_attributes
        supported = bool(attributes & stat.FILE_ATTRIBUTE_READONLY)
        if not supported:
            return False, "setting S_IREAD did not set FILE_ATTRIBUTE_READONLY"
        return True, None
    except OSError as error:
        return False, f"{type(error).__name__}: {error}"
    finally:
        if path.exists():
            os.chmod(path, stat.S_IWRITE)


def probe_named_stream(path: Path) -> tuple[bool, str | None]:
    if os.name != "nt":
        return False, "Windows-only NTFS named-stream probe"
    stream = Path(f"{path}:hasebench")
    payload = b"opaque-stream-value\x00\xff"
    try:
        path.write_bytes(b"base")
        stream.write_bytes(payload)
        if stream.read_bytes() != payload:
            return False, "named stream bytes did not round-trip"
        return True, None
    except OSError as error:
        return False, f"{type(error).__name__}: {error}"


def probe_sparse_file(path: Path) -> tuple[bool, str | None]:
    if os.name != "nt":
        return False, "Windows-only fsutil sparse-range probe"
    with path.open("wb") as stream:
        stream.truncate(1024 * 1024)
    supported, error = command_capability(["fsutil", "sparse", "setflag", str(path)])
    if not supported:
        return False, error
    supported, error = command_capability(
        ["fsutil", "sparse", "setrange", str(path), "0", str(1024 * 1024)]
    )
    if not supported:
        return False, error
    return command_capability(["fsutil", "sparse", "queryflag", str(path)])


def probe(root: Path) -> dict[str, object]:
    root.mkdir(parents=True, exist_ok=True)
    case_path = root / "case-probe"
    case_path.write_bytes(b"case")
    case_sensitive = not (root / "CASE-PROBE").exists()

    hardlink = root / "hardlink-probe"
    hardlinks, hardlink_error = attempt(lambda: os.link(case_path, hardlink))

    symlink = root / "symlink-probe"
    symlinks, symlink_error = attempt(lambda: symlink.symlink_to(case_path.name))

    long_component_root = root
    while len(str(long_component_root)) < 280:
        long_component_root /= "long-path-segment"
    long_paths, long_path_error = attempt(lambda: long_component_root.mkdir(parents=True))

    timestamp = root / "timestamp-probe"
    timestamp.write_bytes(b"time")
    requested_ns = 1_700_000_000_123_456_700
    os.utime(timestamp, ns=(requested_ns, requested_ns))
    observed_ns = timestamp.stat().st_mtime_ns

    readonly, readonly_error = probe_readonly_mapping(root / "readonly-probe")
    named_streams, named_stream_error = probe_named_stream(root / "stream-probe")
    sparse_files, sparse_error = probe_sparse_file(root / "sparse-probe")
    acl_read, acl_error = command_capability(["icacls", str(case_path)])

    return {
        "platform": platform.platform(),
        "os_name": os.name,
        "root": str(root.resolve()),
        "case_sensitive": case_sensitive,
        "hardlinks": {"supported": hardlinks, "error": hardlink_error},
        "symlinks": {"supported": symlinks, "error": symlink_error},
        "long_paths_over_260": {"supported": long_paths, "error": long_path_error},
        "timestamp": {
            "requested_ns": requested_ns,
            "observed_ns": observed_ns,
            "absolute_error_ns": abs(requested_ns - observed_ns),
        },
        "readonly_mapping": {"supported": readonly, "error": readonly_error},
        "named_streams": {"supported": named_streams, "error": named_stream_error},
        "python_xattr_api": {
            "supported": hasattr(os, "getxattr") and hasattr(os, "setxattr"),
            "error": None
            if hasattr(os, "getxattr") and hasattr(os, "setxattr")
            else "Python exposes no native Windows xattr API",
        },
        "sparse_files": {"supported": sparse_files, "error": sparse_error},
        "acl_read": {"supported": acl_read, "error": acl_error},
        "ownership_mapping": {
            "supported": False,
            "error": "no implicit Windows SID to Unix uid/gid mapping",
        },
        "reparse_policy": "unknown tags must not be traversed by default",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--json-output", type=Path)
    arguments = parser.parse_args()
    if arguments.root is None:
        with tempfile.TemporaryDirectory(prefix="hasebench-rsync-win-") as temporary:
            result = probe(Path(temporary))
    else:
        result = probe(arguments.root)
    rendered = json.dumps(result, indent=2, sort_keys=True)
    print(rendered)
    if arguments.json_output is not None:
        arguments.json_output.write_text(rendered + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
