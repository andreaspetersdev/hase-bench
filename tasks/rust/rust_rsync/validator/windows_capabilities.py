#!/usr/bin/env python3
"""Probe Windows filesystem capabilities used by RUST-RSYNC validation."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import tempfile


def attempt(action) -> tuple[bool, str | None]:
    try:
        action()
        return True, None
    except OSError as error:
        return False, f"{type(error).__name__}: {error}"


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
        "python_xattr_api": hasattr(os, "getxattr") and hasattr(os, "setxattr"),
        "sparse_allocation_probe": "not available through portable Python on Windows",
        "acl_probe": "deferred to the platform adapter fixture",
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
