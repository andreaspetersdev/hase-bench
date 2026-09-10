from __future__ import annotations

import json
import shutil
from datetime import datetime
from pathlib import Path

from .models import Task
from .tasks import repository_root

WORKSPACE_METADATA = ".hasebench-workspace.json"


def prepare_workspace(task: Task, work_root: Path | None = None) -> Path:
    base = work_root or repository_root() / "work"
    base.mkdir(parents=True, exist_ok=True)
    stem = f"{datetime.now().strftime('%Y%m%d-%H%M%S')}_{task.identifier}_manual"
    workspace = base / stem
    suffix = 2
    while workspace.exists():
        workspace = base / f"{stem}_{suffix}"
        suffix += 1
    shutil.copytree(task.root / "starter", workspace)
    shutil.copy2(task.root / "TASK.md", workspace / "TASK.md")
    (workspace / WORKSPACE_METADATA).write_text(
        json.dumps({"task_id": task.identifier, "task_version": task.version}, indent=2) + "\n",
        encoding="utf-8",
    )
    return workspace


def task_for_workspace(workspace: Path) -> str:
    metadata = workspace / WORKSPACE_METADATA
    if not metadata.is_file():
        raise ValueError(f"{workspace} is not a prepared Hase Bench workspace")
    return str(json.loads(metadata.read_text(encoding="utf-8"))["task_id"])

