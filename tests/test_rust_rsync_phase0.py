from __future__ import annotations

import csv
import importlib.util
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[1]
RSYNC_ROOT = ROOT / "tasks" / "rust" / "rust_rsync"


def load_fixture_module():
    path = RSYNC_ROOT / "validator" / "fixture_probe.py"
    specification = importlib.util.spec_from_file_location("rust_rsync_fixture_probe", path)
    assert specification is not None and specification.loader is not None
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def test_version_parser_accepts_compatible_rsync_three() -> None:
    module = load_fixture_module()
    assert module.parse_version("rsync  version 3.2.7  protocol version 31\n") == ("3.2.7", 31)
    assert module.parse_version("rsync version 3.5.0 protocol version 32") == ("3.5.0", 32)


def test_version_parser_rejects_old_or_unrecognized_oracles() -> None:
    module = load_fixture_module()
    with pytest.raises(ValueError, match="below required"):
        module.parse_version("rsync version 3.0.9 protocol version 30")
    with pytest.raises(ValueError, match="unable to parse"):
        module.parse_version("not rsync")


def test_compatibility_matrix_is_traceable_and_unique() -> None:
    with (RSYNC_ROOT / "COMPATIBILITY_MATRIX.csv").open(encoding="utf-8", newline="") as stream:
        rows = list(csv.DictReader(stream))
    required = {
        "id",
        "phase",
        "mode",
        "contract",
        "cli",
        "module",
        "visible_test",
        "hidden_or_differential_test",
        "host_rule",
        "status",
    }
    assert rows
    assert set(rows[0]) == required
    identifiers = [row["id"] for row in rows]
    assert len(identifiers) == len(set(identifiers))
    assert {row["mode"] for row in rows} >= {"local", "remote-shell", "daemon"}
    for row in rows:
        assert all(row[column].strip() for column in required)
        assert row["status"] in {"planned", "verified"}


def test_unpublished_capstone_is_not_discoverable() -> None:
    assert not (RSYNC_ROOT / "task.yaml").exists()
