from __future__ import annotations

import json
from pathlib import Path

from .models import Task


def repository_root() -> Path:
    return Path(__file__).resolve().parent.parent


def discover_tasks(root: Path | None = None) -> list[Task]:
    tasks_root = (root or repository_root()) / "tasks"
    discovered: list[Task] = []
    for metadata_path in tasks_root.glob("*/*/task.yaml"):
        raw = json.loads(metadata_path.read_text(encoding="utf-8"))
        discovered.append(
            Task(
                identifier=raw["id"], title=raw["title"], language=raw["language"],
                standard=raw["standard"], difficulty=raw["difficulty"],
                version=int(raw["version"]), root=metadata_path.parent,
            )
        )
    return sorted(discovered, key=lambda task: task.identifier)


def find_task(identifier: str, root: Path | None = None) -> Task:
    normalized = identifier.lower()
    for task in discover_tasks(root):
        if task.identifier == normalized:
            return task
    raise KeyError(f"Unknown task: {identifier}")

