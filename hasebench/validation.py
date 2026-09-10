from __future__ import annotations

import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from .models import Task


@dataclass(frozen=True)
class CommandResult:
    returncode: int
    output: str
    duration_seconds: float
    timed_out: bool = False


@dataclass(frozen=True)
class ValidationResult:
    task: str
    build: CommandResult
    visible: CommandResult | None
    hidden: CommandResult | None
    build_failure_kind: str | None = None

    @property
    def outcome(self) -> str:
        if self.build.returncode != 0:
            return self.build_failure_kind or "BUILD_CONFIGURATION_FAILURE"
        if self.visible is not None and self.visible.returncode != 0:
            return "VISIBLE_TEST_FAILURE"
        if self.hidden is not None and self.hidden.returncode != 0:
            return "HIDDEN_TEST_FAILURE"
        return "SUCCESS"


def _run(arguments: list[str], cwd: Path, timeout: int) -> CommandResult:
    started = time.monotonic()
    try:
        completed = subprocess.run(arguments, cwd=cwd, env=_child_environment(), text=True, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, timeout=timeout, check=False)
        return CommandResult(completed.returncode, completed.stdout, time.monotonic() - started)
    except subprocess.TimeoutExpired as error:
        output = error.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")
        return CommandResult(124, output, time.monotonic() - started, True)


def _child_environment() -> dict[str, str]:
    """Return an environment safe for Windows/MSBuild child processes.

    Windows environment-variable names are case-insensitive.  Some process
    launchers nevertheless supply both ``PATH`` and ``Path``.  MSBuild passes
    that malformed environment to ``cl.exe`` and fails before compilation.
    Keep the first spelling of every variable so subprocesses receive one
    canonical entry per case-insensitive name.
    """
    return _deduplicate_environment(os.environ)


def _deduplicate_environment(source: Mapping[str, str]) -> dict[str, str]:
    environment: dict[str, str] = {}
    names: set[str] = set()
    for name, value in source.items():
        normalized = name.upper()
        if normalized not in names:
            environment[name] = value
            names.add(normalized)
    return environment


def validate_cpp(workspace: Path, task: Task) -> ValidationResult:
    build_dir = workspace / "build" / "hasebench"
    configure = _run(["cmake", "-S", str(workspace), "-B", str(build_dir)], workspace, 120)
    if configure.returncode != 0:
        return ValidationResult(task.identifier, configure, None, None, "BUILD_CONFIGURATION_FAILURE")
    build = _run(["cmake", "--build", str(build_dir), "--config", "Debug"], workspace, 120)
    if build.returncode != 0:
        return ValidationResult(task.identifier, build, None, None, "COMPILATION_FAILURE")
    visible = _run(["ctest", "--test-dir", str(build_dir), "-C", "Debug", "--output-on-failure"], workspace, 60)
    validator_dir = task.root / "validator"
    hidden_build = workspace / "build" / "hasebench-hidden"
    hidden_configure = _run(["cmake", "-S", str(validator_dir), "-B", str(hidden_build),
                             f"-DSTARTER_DIR={workspace}"], workspace, 120)
    if hidden_configure.returncode != 0:
        return ValidationResult(task.identifier, hidden_configure, visible, None, "BUILD_CONFIGURATION_FAILURE")
    hidden_build_result = _run(["cmake", "--build", str(hidden_build), "--config", "Debug"], workspace, 120)
    if hidden_build_result.returncode != 0:
        return ValidationResult(task.identifier, hidden_build_result, visible, None, "COMPILATION_FAILURE")
    hidden = _run(["ctest", "--test-dir", str(hidden_build), "-C", "Debug", "--output-on-failure"], workspace, 90)
    return ValidationResult(task.identifier, configure, visible, hidden)
