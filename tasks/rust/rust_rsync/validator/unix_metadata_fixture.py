#!/usr/bin/env python3
"""Probe Unix metadata semantics with the installed rsync 3.x oracle."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile

from fixture_probe import parse_version


def outcome(status: str, detail: str | None = None) -> dict[str, str]:
    result = {"status": status}
    if detail is not None:
        result["detail"] = detail
    return result


def unavailable(error: BaseException) -> dict[str, str]:
    return outcome("unavailable", f"{type(error).__name__}: {error}")


def run(
    arguments: list[str], *, environment: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30.0,
        env=environment,
    )


def seed_sparse(path: Path) -> bytes:
    head = b"metadata-head\x00"
    tail = b"\xffmetadata-tail"
    with path.open("wb") as output:
        output.write(head)
        output.seek(4 * 1024 * 1024)
        output.write(tail)
    return head + bytes(4 * 1024 * 1024 - len(head)) + tail


def set_access_acl(path: Path) -> bytes:
    # Linux's POSIX ACL xattr format: version, then (tag, perms, id) entries.
    undefined_id = 0xFFFF_FFFF
    named_uid = 65_534 if os.getuid() != 65_534 else 65_533
    entries = [
        (0x01, 0b110, undefined_id),  # ACL_USER_OBJ
        (0x02, 0b100, named_uid),  # ACL_USER
        (0x04, 0b100, undefined_id),  # ACL_GROUP_OBJ
        (0x10, 0b100, undefined_id),  # ACL_MASK
        (0x20, 0, undefined_id),  # ACL_OTHER
    ]
    encoded = struct.pack("<I", 2) + b"".join(
        struct.pack("<HHI", tag, permissions, identifier)
        for tag, permissions, identifier in entries
    )
    os.setxattr(path, "system.posix_acl_access", encoded, follow_symlinks=False)
    return os.getxattr(path, "system.posix_acl_access", follow_symlinks=False)


def allocated_bytes(path: Path) -> int:
    return path.stat().st_blocks * 512


def probe(rsync: str) -> dict[str, object]:
    executable = shutil.which(rsync)
    if executable is None:
        raise FileNotFoundError(f"rsync executable not found: {rsync}")
    banner = run([executable, "--version"]).stdout
    version, protocol = parse_version(banner)
    capabilities: dict[str, dict[str, str]] = {}

    with tempfile.TemporaryDirectory(prefix="hasebench-rsync-unix-meta-") as temporary:
        root = Path(temporary)
        source = root / "source"
        destination = root / "destination"
        source.mkdir()
        destination.mkdir()
        source_file = source / "metadata.bin"
        expected_bytes = seed_sparse(source_file)
        source_file.chmod(0o640)

        capabilities["permissions"] = outcome("supported")
        capabilities["ownership"] = outcome(
            "supported", f"uid={source_file.stat().st_uid}, gid={source_file.stat().st_gid}"
        )

        try:
            os.setxattr(
                source_file,
                "user.hasebench",
                b"opaque\x00unix-xattr",
                follow_symlinks=False,
            )
            expected_xattr = os.getxattr(
                source_file, "user.hasebench", follow_symlinks=False
            )
            capabilities["extended_attributes"] = outcome("supported")
        except (AttributeError, OSError) as error:
            expected_xattr = None
            capabilities["extended_attributes"] = unavailable(error)

        try:
            expected_acl = set_access_acl(source_file)
            capabilities["acls"] = outcome("supported")
        except (AttributeError, OSError) as error:
            expected_acl = None
            capabilities["acls"] = unavailable(error)

        source_allocation = allocated_bytes(source_file)
        if source_allocation < source_file.stat().st_size:
            capabilities["sparse_files"] = outcome(
                "supported", f"source_allocated_bytes={source_allocation}"
            )
        else:
            capabilities["sparse_files"] = outcome(
                "unavailable", "filesystem did not retain a source hole"
            )

        options = ["-a"]
        if expected_acl is not None:
            options.append("-A")
        if expected_xattr is not None:
            options.append("-X")
        if capabilities["sparse_files"]["status"] == "supported":
            options.append("-S")
        run([executable, *options, f"{source}/", f"{destination}/"])

        destination_file = destination / source_file.name
        if destination_file.read_bytes() != expected_bytes:
            raise RuntimeError("rsync metadata fixture changed sparse-file bytes")
        if destination_file.stat().st_mode & 0o7777 != source_file.stat().st_mode & 0o7777:
            raise RuntimeError("rsync did not preserve Unix permission bits")
        if (destination_file.stat().st_uid, destination_file.stat().st_gid) != (
            source_file.stat().st_uid,
            source_file.stat().st_gid,
        ):
            raise RuntimeError("rsync did not preserve representable uid/gid")
        if expected_xattr is not None and os.getxattr(
            destination_file, "user.hasebench", follow_symlinks=False
        ) != expected_xattr:
            raise RuntimeError("rsync did not preserve opaque extended-attribute bytes")
        if expected_acl is not None and os.getxattr(
            destination_file, "system.posix_acl_access", follow_symlinks=False
        ) != expected_acl:
            raise RuntimeError("rsync did not preserve the POSIX access ACL")
        if capabilities["sparse_files"]["status"] == "supported":
            destination_allocation = allocated_bytes(destination_file)
            if destination_allocation >= destination_file.stat().st_size:
                raise RuntimeError("rsync -S did not reduce destination allocation")
            capabilities["sparse_files"]["detail"] = (
                f"source_allocated_bytes={source_allocation}, "
                f"destination_allocated_bytes={destination_allocation}"
            )

        adapter_file = root / "adapter.bin"
        adapter_file.write_bytes(expected_bytes)
        bridge = Path(__file__).resolve().parent.parent / "reference" / "unix_metadata_bridge.py"
        bridge_environment = dict(os.environ)
        bridge_environment.update(
            {
                "HASEBENCH_META_SOURCE": str(source_file),
                "HASEBENCH_META_DESTINATION": str(adapter_file),
                "HASEBENCH_META_OWNER": "1",
                "HASEBENCH_META_GROUP": "1",
            }
        )
        for operation, enabled in (
            ("ownership", True),
            ("xattrs", expected_xattr is not None),
            ("acl", expected_acl is not None),
        ):
            if enabled:
                bridge_environment["HASEBENCH_META_OPERATION"] = operation
                run(["python3", "-I", str(bridge)], environment=bridge_environment)
        if (adapter_file.stat().st_uid, adapter_file.stat().st_gid) != (
            source_file.stat().st_uid,
            source_file.stat().st_gid,
        ):
            raise RuntimeError("Unix metadata bridge did not preserve representable uid/gid")
        if expected_xattr is not None and os.getxattr(
            adapter_file, "user.hasebench", follow_symlinks=False
        ) != expected_xattr:
            raise RuntimeError("Unix metadata bridge did not preserve opaque xattr bytes")
        if expected_acl is not None and os.getxattr(
            adapter_file, "system.posix_acl_access", follow_symlinks=False
        ) != expected_acl:
            raise RuntimeError("Unix metadata bridge did not preserve the POSIX access ACL")

    return {
        "executable": executable,
        "version": version,
        "protocol": protocol,
        "capabilities": capabilities,
        "checks": {
            "unix_metadata_adapter": "pass",
            "unix_metadata_oracle": "pass",
        },
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
