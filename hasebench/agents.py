from __future__ import annotations

import os
import shutil
import subprocess
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Protocol


@dataclass(frozen=True)
class AgentRunRequest:
    workspace: Path
    instruction: str
    model: str
    timeout_seconds: int
    log_path: Path


@dataclass(frozen=True)
class AgentRunResult:
    started_at: str
    ended_at: str
    duration_seconds: float
    exit_code: int | None
    output: str
    outcome: str


class AgentRunner(Protocol):
    """Runs one coding agent without knowing anything about benchmark tasks."""

    def run(self, request: AgentRunRequest) -> AgentRunResult:
        ...


class OpenCodeAgentRunner:
    """Adapter for OpenCode's non-interactive ``run`` command."""

    def __init__(self, executable: str | None = None) -> None:
        self.executable = executable or _default_opencode_executable()

    def run(self, request: AgentRunRequest) -> AgentRunResult:
        started_at = _timestamp()
        started = time.monotonic()
        arguments = [
            self.executable,
            "run",
            "--dir",
            str(request.workspace),
            "--model",
            request.model,
            "--format",
            "json",
            "--auto",
            request.instruction,
        ]
        environment = _agent_environment(request.workspace)
        try:
            completed = subprocess.run(
                arguments,
                cwd=request.workspace,
                env=environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=request.timeout_seconds,
                check=False,
            )
            output = completed.stdout
            outcome = "SUCCESS" if completed.returncode == 0 else "AGENT_NONZERO_EXIT"
            result = AgentRunResult(
                started_at, _timestamp(), time.monotonic() - started, completed.returncode, output, outcome
            )
        except subprocess.TimeoutExpired as error:
            output = error.stdout or ""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            result = AgentRunResult(started_at, _timestamp(), time.monotonic() - started, None, output, "AGENT_TIMEOUT")
        except OSError as error:
            result = AgentRunResult(started_at, _timestamp(), time.monotonic() - started, None, str(error), "AGENT_LAUNCH_FAILURE")

        request.log_path.write_text(result.output, encoding="utf-8")
        return result


def _timestamp() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _default_opencode_executable() -> str:
    """Use npm's executable command wrapper on Windows, not its PowerShell shim."""
    if os.name == "nt":
        return shutil.which("opencode.cmd") or "opencode.cmd"
    return "opencode"


def _agent_environment(workspace: Path) -> dict[str, str]:
    """Keep ordinary agent-created temporary files inside the saved workspace."""
    temporary = workspace / ".hasebench-tmp"
    temporary.mkdir(exist_ok=True)
    environment = dict(os.environ)
    environment["TMP"] = str(temporary)
    environment["TEMP"] = str(temporary)
    return environment
