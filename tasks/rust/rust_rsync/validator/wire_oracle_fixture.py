#!/usr/bin/env python3
"""Capture upstream rsync's remote-shell protocol greeting in both roles."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from fixture_probe import parse_version


def run(arguments: list[str], *, environment: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=10.0,
        env=environment,
    )


def capture_role(
    executable: str,
    shim: Path,
    capture: Path,
    arguments: list[str],
) -> str:
    environment = dict(os.environ)
    environment["HASEBENCH_WIRE_CAPTURE"] = str(capture)
    result = run([executable, "-e", str(shim), *arguments], environment=environment)
    if result.returncode == 0:
        raise RuntimeError("truncated fake peer unexpectedly completed an rsync transfer")
    greeting = capture.read_bytes()
    expected = (31).to_bytes(4, "little")
    if greeting != expected:
        raise RuntimeError(
            f"unexpected protocol greeting: expected={expected.hex()} actual={greeting.hex()}"
        )
    return greeting.hex()


def probe(rsync: str) -> dict[str, object]:
    executable = shutil.which(rsync)
    if executable is None:
        raise FileNotFoundError(f"rsync executable not found: {rsync}")
    banner = subprocess.run(
        [executable, "--version"],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=10.0,
    ).stdout
    version, protocol = parse_version(banner)

    with tempfile.TemporaryDirectory(prefix="hasebench-rsync-wire-") as temporary:
        root = Path(temporary)
        source = root / "source"
        destination = root / "destination"
        source.mkdir()
        destination.mkdir()
        (source / "payload").write_bytes(b"wire greeting")
        peer = root / "fake-peer.py"
        peer.write_text(
            "import os, pathlib, sys\n"
            "greeting = sys.stdin.buffer.read(4)\n"
            "pathlib.Path(os.environ['HASEBENCH_WIRE_CAPTURE']).write_bytes(greeting)\n"
            "sys.stdout.buffer.write((31).to_bytes(4, 'little'))\n"
            "sys.stdout.buffer.flush()\n",
            encoding="utf-8",
        )
        shim = root / "remote-shell"
        shim.write_text(
            f"#!/bin/sh\nexec python3 {peer}\n",
            encoding="utf-8",
        )
        shim.chmod(0o700)
        push = capture_role(
            executable,
            shim,
            root / "push.bin",
            ["-a", f"{source}/", "loopback:/destination/"],
        )
        pull = capture_role(
            executable,
            shim,
            root / "pull.bin",
            ["-a", "loopback:/source/", f"{destination}/"],
        )

    return {
        "executable": executable,
        "version": version,
        "protocol": protocol,
        "greeting_hex": {"push": push, "pull": pull},
        "checks": {"remote_shell_protocol_greeting": "pass"},
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rsync", default="rsync")
    parser.add_argument("--json-output", type=Path)
    arguments = parser.parse_args()
    result = probe(arguments.rsync)
    rendered = json.dumps(result, indent=2, sort_keys=True)
    print(rendered)
    if arguments.json_output is not None:
        arguments.json_output.write_text(rendered + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
