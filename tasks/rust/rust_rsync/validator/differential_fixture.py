#!/usr/bin/env python3
"""Compare a candidate rsync-compatible executable with an upstream oracle."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import shlex
import socket
import subprocess
import tempfile
import time

from fixture_probe import assert_same, free_loopback_port, run, seed_tree


def reset_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def shell_shim(path: Path, server: str) -> None:
    quoted = shlex.quote(server)
    path.write_text(
        f'#!/bin/sh\nshift\nif [ "$1" = rsync ]; then shift; fi\nexec {quoted} "$@"\n',
        encoding="utf-8",
    )
    path.chmod(0o700)


def probe_local(candidate: str, oracle: str, root: Path, source: Path) -> dict[str, str]:
    oracle_destination = root / "local-oracle"
    candidate_destination = root / "local-candidate"
    reset_directory(oracle_destination)
    reset_directory(candidate_destination)
    run([oracle, "-a", f"{source}/", f"{oracle_destination}/"])
    run([candidate, "-a", f"{source}/", f"{candidate_destination}/"])
    assert_same(oracle_destination, candidate_destination, "local differential")

    (oracle_destination / "delete-me").write_bytes(b"old")
    (candidate_destination / "delete-me").write_bytes(b"old")
    run([oracle, "-a", "--delete", f"{source}/", f"{oracle_destination}/"])
    run([candidate, "-a", "--delete", f"{source}/", f"{candidate_destination}/"])
    assert_same(oracle_destination, candidate_destination, "local delete differential")

    before = list(candidate_destination.rglob("*"))
    candidate_dry_run = run([candidate, "-a", "--dry-run", f"{source}/", f"{candidate_destination}/"])
    after = list(candidate_destination.rglob("*"))
    if [item.relative_to(candidate_destination) for item in before] != [
        item.relative_to(candidate_destination) for item in after
    ]:
        raise RuntimeError("candidate dry-run changed destination entries")
    return {"archive_delete": "pass", "dry_run": "pass", "dry_run_output": candidate_dry_run.stdout}


def probe_remote_shell(candidate: str, oracle: str, root: Path, source: Path) -> dict[str, str]:
    oracle_server = root / "oracle-shell"
    candidate_server = root / "candidate-shell"
    shell_shim(oracle_server, oracle)
    shell_shim(candidate_server, candidate)

    candidate_push = root / "shell-candidate-push"
    candidate_pull = root / "shell-candidate-pull"
    oracle_push = root / "shell-oracle-push"
    oracle_pull = root / "shell-oracle-pull"
    for directory in (candidate_push, candidate_pull, oracle_push, oracle_pull):
        reset_directory(directory)

    run([candidate, "-a", "-e", str(oracle_server), f"{source}/", f"loopback:{candidate_push}/"])
    assert_same(source, candidate_push, "candidate client push to oracle server")
    run([candidate, "-a", "-e", str(oracle_server), f"loopback:{source}/", f"{candidate_pull}/"])
    assert_same(source, candidate_pull, "candidate client pull from oracle server")

    run([oracle, "-a", "-e", str(candidate_server), f"{source}/", f"loopback:{oracle_push}/"])
    assert_same(source, oracle_push, "oracle client push to candidate server")
    run([oracle, "-a", "-e", str(candidate_server), f"loopback:{source}/", f"{oracle_pull}/"])
    assert_same(source, oracle_pull, "oracle client pull from candidate server")
    return {
        "candidate_client_push": "pass",
        "candidate_client_pull": "pass",
        "candidate_server_receive": "pass",
        "candidate_server_send": "pass",
    }


def wait_for_port(port: int, process: subprocess.Popen[str]) -> None:
    deadline = time.monotonic() + 5.0
    while time.monotonic() < deadline:
        if process.poll() is not None:
            output, _ = process.communicate()
            raise RuntimeError(f"daemon exited early ({process.returncode}):\n{output}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.05)
    raise TimeoutError("daemon did not listen on loopback")


def start_daemon(program: str, root: Path, name: str) -> tuple[subprocess.Popen[str], int, Path]:
    module_root = root / f"{name}-module"
    module_root.mkdir()
    port = free_loopback_port()
    config = root / f"{name}.conf"
    config.write_text(
        "\n".join(
            [
                "address = 127.0.0.1",
                "use chroot = false",
                f"pid file = {root / f'{name}.pid'}",
                f"log file = {root / f'{name}.log'}",
                "[fixture]",
                f"path = {module_root}",
                "read only = false",
                "munge symlinks = false",
                "",
            ]
        ),
        encoding="utf-8",
    )
    process = subprocess.Popen(
        [program, "--daemon", "--no-detach", f"--config={config}", f"--port={port}"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    wait_for_port(port, process)
    return process, port, module_root


def stop_daemon(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=5.0)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5.0)


def probe_daemon(candidate: str, oracle: str, root: Path, source: Path) -> dict[str, str]:
    checks: dict[str, str] = {}
    oracle_process, oracle_port, oracle_module = start_daemon(oracle, root, "oracle-daemon")
    try:
        oracle_url = f"rsync://127.0.0.1:{oracle_port}/fixture/"
        run([candidate, "-a", f"{source}/", oracle_url])
        assert_same(source, oracle_module, "candidate client daemon push")
        candidate_pull = root / "candidate-daemon-pull"
        candidate_pull.mkdir()
        run([candidate, "-a", oracle_url, f"{candidate_pull}/"])
        assert_same(source, candidate_pull, "candidate client daemon pull")
        checks["candidate_client_push_pull"] = "pass"
    finally:
        stop_daemon(oracle_process)

    candidate_process, candidate_port, candidate_module = start_daemon(candidate, root, "candidate-daemon")
    try:
        candidate_url = f"rsync://127.0.0.1:{candidate_port}/fixture/"
        run([oracle, "-a", f"{source}/", candidate_url])
        assert_same(source, candidate_module, "candidate daemon receive")
        oracle_pull = root / "oracle-daemon-pull"
        oracle_pull.mkdir()
        run([oracle, "-a", candidate_url, f"{oracle_pull}/"])
        assert_same(source, oracle_pull, "candidate daemon send")
        checks["candidate_server_send_receive"] = "pass"
    finally:
        stop_daemon(candidate_process)
    return checks


def differential(candidate: str, oracle: str) -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="hasebench-rsync-diff-") as temporary:
        root = Path(temporary)
        source = root / "source"
        source.mkdir()
        seed_tree(source)
        return {
            "candidate": candidate,
            "oracle": oracle,
            "local": probe_local(candidate, oracle, root, source),
            "remote_shell": probe_remote_shell(candidate, oracle, root, source),
            "daemon": probe_daemon(candidate, oracle, root, source),
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--oracle", default="rsync")
    parser.add_argument("--json-output", type=Path)
    arguments = parser.parse_args()
    result = differential(arguments.candidate, arguments.oracle)
    rendered = json.dumps(result, indent=2, sort_keys=True)
    print(rendered)
    if arguments.json_output is not None:
        arguments.json_output.write_text(rendered + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
