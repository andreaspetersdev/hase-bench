#!/usr/bin/env python3
"""Tool-independent Unix metadata operations embedded by the Rust reference."""

from __future__ import annotations

import errno
import os


def copy_ownership(source: str, destination: str) -> None:
    metadata = os.stat(source, follow_symlinks=False)
    owner = metadata.st_uid if os.environ["HASEBENCH_META_OWNER"] == "1" else -1
    group = metadata.st_gid if os.environ["HASEBENCH_META_GROUP"] == "1" else -1
    os.chown(destination, owner, group, follow_symlinks=False)


def copy_acl(source: str, destination: str) -> None:
    names = ("system.posix_acl_access", "system.posix_acl_default")
    source_names = set(os.listxattr(source, follow_symlinks=False))
    for name in names:
        if name in source_names:
            value = os.getxattr(source, name, follow_symlinks=False)
            os.setxattr(destination, name, value, follow_symlinks=False)
        else:
            try:
                os.removexattr(destination, name, follow_symlinks=False)
            except OSError as error:
                if error.errno not in (errno.ENODATA, errno.ENOTSUP):
                    raise


def ordinary_xattrs(path: str) -> set[str]:
    return {
        name
        for name in os.listxattr(path, follow_symlinks=False)
        if not name.startswith("system.posix_acl_")
    }


def copy_xattrs(source: str, destination: str) -> None:
    source_names = ordinary_xattrs(source)
    destination_names = ordinary_xattrs(destination)
    for name in destination_names - source_names:
        try:
            os.removexattr(destination, name, follow_symlinks=False)
        except OSError as error:
            if error.errno not in (errno.ENODATA, errno.ENOTSUP):
                raise
    for name in source_names:
        value = os.getxattr(source, name, follow_symlinks=False)
        os.setxattr(destination, name, value, follow_symlinks=False)


def main() -> None:
    source = os.environ["HASEBENCH_META_SOURCE"]
    destination = os.environ["HASEBENCH_META_DESTINATION"]
    operation = os.environ["HASEBENCH_META_OPERATION"]
    if operation == "ownership":
        copy_ownership(source, destination)
    elif operation == "acl":
        copy_acl(source, destination)
    elif operation == "xattrs":
        copy_xattrs(source, destination)
    else:
        raise ValueError(f"unknown metadata operation: {operation}")


if __name__ == "__main__":
    main()
