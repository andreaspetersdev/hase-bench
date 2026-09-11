from __future__ import annotations

import json
import re
import shutil
from datetime import datetime
from dataclasses import dataclass
from pathlib import Path

from .models import Task
from .tasks import repository_root

WORKSPACE_METADATA = ".hasebench-workspace.json"


@dataclass(frozen=True)
class WorkspaceCandidate:
    """A direct child of ``work/`` marked as a benchmark workspace.

    ``metadata_error`` is retained instead of dropping malformed workspaces so
    batch validation can report the problem and continue with other runs.
    """

    path: Path
    task_id: str | None
    workspace_id: str | None
    metadata_error: str | None = None


def prepare_workspace(task: Task, work_root: Path | None = None, label: str | None = None) -> Path:
    base = work_root or repository_root() / "work"
    base.mkdir(parents=True, exist_ok=True)
    normalized_label = _normalize_label(label)
    stem = f"{datetime.now().strftime('%Y%m%d-%H%M%S')}_{task.identifier}_man"
    if normalized_label:
        stem += f"_{normalized_label}"
    workspace = base / stem
    suffix = 2
    while workspace.exists():
        workspace = base / f"{stem}_{suffix}"
        suffix += 1
    shutil.copytree(task.root / "starter", workspace)
    shutil.copy2(task.root / "TASK.md", workspace / "TASK.md")
    (workspace / WORKSPACE_METADATA).write_text(
        json.dumps(
            {
                "workspace_id": workspace.name,
                "task_id": task.identifier,
                "task_version": task.version,
            },
            indent=2,
        ) + "\n",
        encoding="utf-8",
    )
    return workspace


def _normalize_label(label: str | None) -> str | None:
    if label is None:
        return None
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", label):
        raise ValueError("workspace label must contain only letters, digits, '.', '_', or '-' and begin with a letter or digit")
    return label


def task_for_workspace(workspace: Path) -> str:
    metadata = workspace / WORKSPACE_METADATA
    if not metadata.is_file():
        raise ValueError(f"{workspace} is not a prepared Hase Bench workspace")
    return str(json.loads(metadata.read_text(encoding="utf-8"))["task_id"])


def discover_workspaces(work_root: Path | None = None) -> list[WorkspaceCandidate]:
    """Find only direct work-directory children carrying our metadata marker."""
    base = work_root or repository_root() / "work"
    if not base.is_dir():
        return []

    candidates: list[WorkspaceCandidate] = []
    for path in sorted((item for item in base.iterdir() if item.is_dir()), key=lambda item: item.name):
        metadata_path = path / WORKSPACE_METADATA
        if not metadata_path.is_file():
            continue
        try:
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
            if not isinstance(metadata, dict):
                raise ValueError("metadata must be a JSON object")
            task_id = metadata.get("task_id")
            if not isinstance(task_id, str) or not task_id:
                raise ValueError("missing a non-empty task_id")
            task_version = metadata.get("task_version")
            if not isinstance(task_version, int) or isinstance(task_version, bool):
                raise ValueError("missing an integer task_version")
            workspace_id = metadata.get("workspace_id", path.name)
            if not isinstance(workspace_id, str) or not workspace_id:
                raise ValueError("missing a non-empty workspace_id")
            candidates.append(WorkspaceCandidate(path, task_id, workspace_id))
        except (OSError, ValueError, json.JSONDecodeError) as error:
            candidates.append(WorkspaceCandidate(path, None, None, f"invalid workspace metadata: {error}"))
    return candidates
