#!/usr/bin/env python3
"""Compare a Windows candidate with WSL rsync for local-transfer semantics."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from fixture_probe import assert_same


def run(arguments: list[str], *, timeout: float = 30.0) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
    )


def wsl_path(path: Path, wsl: str) -> str:
    del wsl
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"fixture path is not on a Windows drive: {resolved}")
    relative = str(resolved)[len(drive) :].lstrip("\\/").replace("\\", "/")
    return f"/mnt/{drive[0].lower()}/{relative}"


def with_trailing_separator(path: Path) -> str:
    return str(path) + os.sep


def wsl_trailing(path: Path, wsl: str) -> str:
    return wsl_path(path, wsl) + "/"


def seed_tree(root: Path) -> None:
    (root / "nested").mkdir(parents=True)
    (root / "hello.txt").write_text("hello rsync\n", encoding="utf-8")
    (root / "empty").write_bytes(b"")
    (root / "name with spaces.bin").write_bytes(bytes(range(256)) * 3)
    (root / "nested" / "payload.bin").write_bytes(b"prefix\x00middle\xffsuffix")


def compare_contents(candidate: str, wsl: str, root: Path, source: Path) -> dict[str, str]:
    candidate_destination = root / "candidate-contents"
    oracle_destination = root / "oracle-contents"
    candidate_destination.mkdir()
    oracle_destination.mkdir()
    run([candidate, "-a", with_trailing_separator(source), with_trailing_separator(candidate_destination)])
    run([wsl, "rsync", "-a", wsl_trailing(source, wsl), wsl_trailing(oracle_destination, wsl)])
    assert_same(oracle_destination, candidate_destination, "Windows/WSL contents differential")

    (candidate_destination / "delete-me").write_bytes(b"old")
    (oracle_destination / "delete-me").write_bytes(b"old")
    run(
        [
            candidate,
            "-a",
            "--delete",
            with_trailing_separator(source),
            with_trailing_separator(candidate_destination),
        ]
    )
    run(
        [
            wsl,
            "rsync",
            "-a",
            "--delete",
            wsl_trailing(source, wsl),
            wsl_trailing(oracle_destination, wsl),
        ]
    )
    assert_same(oracle_destination, candidate_destination, "Windows/WSL delete differential")

    before = sorted(path.relative_to(candidate_destination) for path in candidate_destination.rglob("*"))
    run(
        [
            candidate,
            "-an",
            with_trailing_separator(source),
            with_trailing_separator(candidate_destination),
        ]
    )
    after = sorted(path.relative_to(candidate_destination) for path in candidate_destination.rglob("*"))
    if before != after:
        raise RuntimeError("candidate dry-run changed destination entries")
    return {"archive_contents": "pass", "delete": "pass", "dry_run": "pass"}


def compare_directory_shape(candidate: str, wsl: str, root: Path, source: Path) -> dict[str, str]:
    candidate_absent = root / "candidate-renamed"
    oracle_absent = root / "oracle-renamed"
    run([candidate, "-a", str(source), str(candidate_absent)])
    run([wsl, "rsync", "-a", wsl_path(source, wsl), wsl_path(oracle_absent, wsl)])
    assert_same(oracle_absent, candidate_absent, "Windows/WSL absent destination differential")

    candidate_existing = root / "candidate-existing"
    oracle_existing = root / "oracle-existing"
    candidate_existing.mkdir()
    oracle_existing.mkdir()
    run([candidate, "-a", str(source), str(candidate_existing)])
    run([wsl, "rsync", "-a", wsl_path(source, wsl), wsl_path(oracle_existing, wsl)])
    assert_same(oracle_existing, candidate_existing, "Windows/WSL existing destination differential")
    return {"absent_destination": "pass", "existing_destination": "pass"}


def compare_multiple_sources(candidate: str, wsl: str, root: Path) -> str:
    first = root / "first.bin"
    second = root / "second.bin"
    first.write_bytes(b"one")
    second.write_bytes(b"two")
    candidate_destination = root / "candidate-multiple"
    oracle_destination = root / "oracle-multiple"
    candidate_destination.mkdir()
    oracle_destination.mkdir()
    run([candidate, "-av", str(first), str(second), str(candidate_destination)])
    run(
        [
            wsl,
            "rsync",
            "-av",
            wsl_path(first, wsl),
            wsl_path(second, wsl),
            wsl_path(oracle_destination, wsl),
        ]
    )
    assert_same(oracle_destination, candidate_destination, "Windows/WSL multiple-source differential")
    return "pass"


def differential(candidate: str, wsl: str) -> dict[str, object]:
    candidate_path = shutil.which(candidate) or candidate
    if not Path(candidate_path).is_file():
        raise FileNotFoundError(f"candidate executable not found: {candidate}")
    run([wsl, "rsync", "--version"])
    with tempfile.TemporaryDirectory(prefix="hasebench-rsync-cross-") as temporary:
        root = Path(temporary)
        source = root / "source"
        source.mkdir()
        seed_tree(source)
        return {
            "candidate": str(Path(candidate_path).resolve()),
            "oracle": "WSL rsync",
            "contents": compare_contents(candidate_path, wsl, root, source),
            "directory_shape": compare_directory_shape(candidate_path, wsl, root, source),
            "multiple_sources": compare_multiple_sources(candidate_path, wsl, root),
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--wsl", default="wsl.exe")
    parser.add_argument("--json-output", type=Path)
    arguments = parser.parse_args()
    result = differential(arguments.candidate, arguments.wsl)
    rendered = json.dumps(result, indent=2, sort_keys=True)
    print(rendered)
    if arguments.json_output is not None:
        arguments.json_output.write_text(rendered + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
