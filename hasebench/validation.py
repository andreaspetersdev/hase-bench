from __future__ import annotations

import json
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


def _cmake_debug_flags() -> str | None:
    """Force benchmark executables to report MSVC assertions non-interactively."""
    if os.name != "nt":
        return None
    header = (Path(__file__).with_name("msvc_noninteractive_assert.hpp")).resolve()
    # CMake forwards this value to cl.exe.  Quote the path so installations in
    # directories containing spaces remain valid.
    return f'/FI"{header}"'


def _configure_arguments(source: Path, build_dir: Path, *, extra: list[str] | None = None) -> list[str]:
    arguments = ["cmake", "-S", str(source), "-B", str(build_dir)]
    flags = _cmake_debug_flags()
    if flags is not None:
        arguments.append(f"-DCMAKE_CXX_FLAGS_DEBUG={flags}")
    if extra:
        arguments.extend(extra)
    return arguments


def validate_cpp(workspace: Path, task: Task) -> ValidationResult:
    build_dir = workspace / "build" / "hasebench"
    configure = _run(_configure_arguments(workspace, build_dir), workspace, 120)
    if configure.returncode != 0:
        return ValidationResult(task.identifier, configure, None, None, "BUILD_CONFIGURATION_FAILURE")
    build = _run(["cmake", "--build", str(build_dir), "--config", "Debug"], workspace, 120)
    if build.returncode != 0:
        return ValidationResult(task.identifier, build, None, None, "COMPILATION_FAILURE")
    visible = _run(["ctest", "--test-dir", str(build_dir), "-C", "Debug", "--output-on-failure"], workspace, 60)
    validator_dir = task.root / "validator"
    hidden_build = workspace / "build" / "hasebench-hidden"
    hidden_configure = _run(_configure_arguments(validator_dir, hidden_build,
                                                 extra=[f"-DSTARTER_DIR={workspace}"]), workspace, 120)
    if hidden_configure.returncode != 0:
        return ValidationResult(task.identifier, hidden_configure, visible, None, "BUILD_CONFIGURATION_FAILURE")
    hidden_build_result = _run(["cmake", "--build", str(hidden_build), "--config", "Debug"], workspace, 120)
    if hidden_build_result.returncode != 0:
        return ValidationResult(task.identifier, hidden_build_result, visible, None, "COMPILATION_FAILURE")
    hidden = _run(["ctest", "--test-dir", str(hidden_build), "-C", "Debug", "--output-on-failure"], workspace, 90)
    return ValidationResult(task.identifier, configure, visible, hidden)


def validate_rust(workspace: Path, task: Task) -> ValidationResult:
    manifest = workspace / "Cargo.toml"
    build_dir = workspace / "build" / "hasebench"
    build = _run(
        ["cargo", "build", "--locked", "--manifest-path", str(manifest), "--target-dir", str(build_dir)],
        workspace,
        120,
    )
    if build.returncode != 0:
        return ValidationResult(task.identifier, build, None, None, "COMPILATION_FAILURE")

    visible = _run(
        ["cargo", "test", "--locked", "--manifest-path", str(manifest), "--target-dir", str(build_dir)],
        workspace,
        60,
    )

    validator_manifest = task.root / "validator" / "Cargo.toml"
    hidden_build_dir = workspace / "build" / "hasebench-hidden"
    workspace_path = json.dumps(workspace.resolve().as_posix())
    workspace_patch = f"patch.crates-io.{task.identifier}.path={workspace_path}"
    hidden = _run(
        [
            "cargo",
            "test",
            "--locked",
            "--manifest-path",
            str(validator_manifest),
            "--target-dir",
            str(hidden_build_dir),
            "--config",
            workspace_patch,
        ],
        workspace,
        90,
    )
    return ValidationResult(task.identifier, build, visible, hidden)


def validate_task(workspace: Path, task: Task) -> ValidationResult:
    """Validate a task with the implementation registered for its language."""
    validators = {
        "cpp": validate_cpp,
        "rust": validate_rust,
    }
    try:
        validator = validators[task.language]
    except KeyError as error:
        raise ValueError(f"No validator is available for {task.language}") from error
    return validator(workspace, task)
