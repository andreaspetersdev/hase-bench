from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Task:
    identifier: str
    title: str
    language: str
    standard: str
    difficulty: str
    version: int
    root: Path

