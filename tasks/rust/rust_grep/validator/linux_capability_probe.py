#!/usr/bin/env python3
"""Linux-only Phase 0 filesystem capability probe; not a benchmark validator."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path


def main() -> int:
    if os.name != "posix":
        print("SKIP: Linux capability probe requires POSIX")
        return 0
    with tempfile.TemporaryDirectory(prefix="rust-grep-linux-") as raw:
        root = Path(raw)
        target = root / "target"
        target.mkdir()
        link = root / "link"
        os.symlink(target, link, target_is_directory=True)
        target_stat = target.stat()
        link_stat = link.stat()
        if (target_stat.st_dev, target_stat.st_ino) != (link_stat.st_dev, link_stat.st_ino):
            print("FAIL: symlink target identity was not stable")
            return 1
        raw_name = os.fsencode(root) + b"/nonutf8-\xff"
        descriptor = os.open(raw_name, os.O_CREAT | os.O_WRONLY, 0o600)
        os.close(descriptor)
        if not os.path.exists(raw_name):
            print("FAIL: byte filename was not retained")
            return 1
        print("symlink_loop_identity=available")
        print("non_utf8_filename=available")
        print("effective_uid=" + str(os.geteuid()))
    for name in ("grep", "rustc", "cargo"):
        located = shutil.which(name)
        print(f"{name}=" + (located or "unavailable"))
        if located and name in {"rustc", "cargo"}:
            result = subprocess.run([located, "--version"], stdout=subprocess.PIPE,
                                    stderr=subprocess.PIPE, check=False)
            print(result.stdout.decode("utf-8", "replace").strip())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
