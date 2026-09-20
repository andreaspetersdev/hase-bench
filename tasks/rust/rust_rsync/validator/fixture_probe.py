#!/usr/bin/env python3
"""Self-test an installed rsync 3.x oracle in all required transfer modes."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import socket
import stat
import subprocess
import tempfile
import time


VERSION_PATTERN = re.compile(r"rsync\s+version\s+(3\.\d+(?:\.\d+)?)\s+protocol version\s+(\d+)")


def parse_version(banner: str) -> tuple[str, int]:
    match = VERSION_PATTERN.search(banner)
    if match is None:
        raise ValueError("unable to parse rsync version and protocol")
    version, protocol_text = match.groups()
    protocol = int(protocol_text)
    if protocol < 31:
        raise ValueError(f"rsync protocol {protocol} is below required protocol 31")
    return version, protocol


def run(arguments: list[str], *, timeout: float = 20.0) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
    )


def tree_manifest(root: Path) -> list[dict[str, object]]:
    entries: list[dict[str, object]] = []
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        relative = path.relative_to(root).as_posix()
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            entries.append({"path": relative, "kind": "symlink", "target": os.readlink(path)})
        elif stat.S_ISDIR(metadata.st_mode):
            entries.append({"path": relative, "kind": "directory"})
        elif stat.S_ISREG(metadata.st_mode):
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            entries.append({"path": relative, "kind": "file", "size": metadata.st_size, "sha256": digest})
        else:
            entries.append({"path": relative, "kind": "other", "mode": stat.S_IFMT(metadata.st_mode)})
    return entries


def seed_tree(root: Path) -> None:
    (root / "nested").mkdir(parents=True)
    (root / "hello.txt").write_text("hello rsync\n", encoding="utf-8")
    (root / "empty").write_bytes(b"")
    (root / "name with spaces.bin").write_bytes(bytes(range(256)) * 3)
    (root / "nested" / "payload.bin").write_bytes(b"prefix\x00middle\xffsuffix")
    try:
        (root / "relative-link").symlink_to("hello.txt")
    except (NotImplementedError, OSError):
        pass


def assert_same(expected: Path, actual: Path, label: str) -> None:
    expected_manifest = tree_manifest(expected)
    actual_manifest = tree_manifest(actual)
    if expected_manifest != actual_manifest:
        raise RuntimeError(
            f"{label} manifest mismatch:\nexpected={expected_manifest!r}\nactual={actual_manifest!r}"
        )


def free_loopback_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def wait_for_port(port: int, process: subprocess.Popen[str]) -> None:
    deadline = time.monotonic() + 5.0
    while time.monotonic() < deadline:
        if process.poll() is not None:
            output, _ = process.communicate()
            raise RuntimeError(f"rsync daemon exited early ({process.returncode}):\n{output}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.05)
    raise TimeoutError("rsync daemon did not begin listening on loopback")


def probe_local(rsync: str, root: Path, source: Path) -> None:
    destination = root / "local-destination"
    destination.mkdir()
    run([rsync, "-a", f"{source}/", f"{destination}/"])
    assert_same(source, destination, "local")


def probe_remote_shell(rsync: str, root: Path, source: Path) -> None:
    shim = root / "loopback-shell"
    shim.write_text("#!/bin/sh\nshift\nexec \"$@\"\n", encoding="utf-8")
    shim.chmod(0o700)
    remote = root / "shell-remote"
    pulled = root / "shell-pulled"
    remote.mkdir()
    pulled.mkdir()
    run([rsync, "-a", "-e", str(shim), f"{source}/", f"loopback:{remote}/"])
    assert_same(source, remote, "remote-shell push")
    run([rsync, "-a", "-e", str(shim), f"loopback:{remote}/", f"{pulled}/"])
    assert_same(source, pulled, "remote-shell pull")


def probe_daemon(rsync: str, root: Path, source: Path) -> None:
    module_root = root / "daemon-module"
    pulled = root / "daemon-pulled"
    module_root.mkdir()
    pulled.mkdir()
    port = free_loopback_port()
    config = root / "rsyncd.conf"
    config.write_text(
        "\n".join(
            [
                "address = 127.0.0.1",
                "use chroot = false",
                f"pid file = {root / 'rsyncd.pid'}",
                f"log file = {root / 'rsyncd.log'}",
                "[fixture]",
                f"path = {module_root}",
                "read only = false",
                "munge symlinks = false",
                "list = true",
                "",
            ]
        ),
        encoding="utf-8",
    )
    process = subprocess.Popen(
        [rsync, "--daemon", "--no-detach", f"--config={config}", f"--port={port}"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    try:
        wait_for_port(port, process)
        url = f"rsync://127.0.0.1:{port}/fixture/"
        run([rsync, "-a", f"{source}/", url])
        assert_same(source, module_root, "daemon push")
        run([rsync, "-a", url, f"{pulled}/"])
        assert_same(source, pulled, "daemon pull")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5.0)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5.0)


def probe(rsync: str) -> dict[str, object]:
    executable = shutil.which(rsync)
    if executable is None:
        raise FileNotFoundError(f"rsync executable not found: {rsync}")
    banner = run([executable, "--version"]).stdout
    version, protocol = parse_version(banner)
    checks: dict[str, str] = {}
    with tempfile.TemporaryDirectory(prefix="hasebench-rsync-") as temporary:
        root = Path(temporary)
        source = root / "source"
        source.mkdir()
        seed_tree(source)
        for name, action in (
            ("local", probe_local),
            ("remote_shell", probe_remote_shell),
            ("daemon", probe_daemon),
        ):
            action(executable, root, source)
            checks[name] = "pass"
    return {
        "executable": executable,
        "version": version,
        "protocol": protocol,
        "banner": banner.rstrip(),
        "checks": checks,
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
