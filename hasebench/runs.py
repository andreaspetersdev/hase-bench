from __future__ import annotations

import json
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

from .agents import AgentRunRequest, AgentRunResult, AgentRunner
from .models import Task
from .validation import CommandResult, ValidationResult, validate_cpp
from .workspaces import prepare_workspace

AUTONOMOUS_INSTRUCTION = """Read TASK.md and solve the task.

The current directory is the entire project scope for this task. Do not inspect, list, read, or modify parent or ancestor directories, even if they are visible through Git. Do not change files outside the current workspace.

You may inspect the repository, edit the implementation, configure/build the project, and run the visible tests.

Continue until you believe the task is complete."""

RUN_METADATA = "hasebench-run.json"
AGENT_LOG = "hasebench-agent.log"


@dataclass(frozen=True)
class AutonomousRunResult:
    workspace: Path
    agent: AgentRunResult
    validation: ValidationResult
    outcome: str
    total_duration_seconds: float = 0.0


def run_autonomous(
    task: Task,
    runner: AgentRunner,
    model_configuration: str,
    model_name: str,
    backend: str,
    timeout_seconds: int,
    label: str | None = None,
) -> AutonomousRunResult:
    started = time.monotonic()
    workspace = prepare_workspace(task, mode="aut_opencode", label=label)
    agent = runner.run(
        AgentRunRequest(workspace, AUTONOMOUS_INSTRUCTION, model_configuration, timeout_seconds, workspace / AGENT_LOG)
    )
    validation = validate_cpp(workspace, task)
    outcome = agent.outcome if agent.outcome != "SUCCESS" else validation.outcome
    result = AutonomousRunResult(workspace, agent, validation, outcome, time.monotonic() - started)
    _write_metadata(result, task, model_configuration, model_name, backend, timeout_seconds)
    return result


def _write_metadata(
    result: AutonomousRunResult,
    task: Task,
    model_configuration: str,
    model_name: str,
    backend: str,
    timeout_seconds: int,
) -> None:
    metadata = {
        "schema_version": 1,
        "run_id": result.workspace.name,
        "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "mode": "autonomous",
        "task": {"id": task.identifier, "version": task.version, "language": task.language},
        "agent": {"name": "opencode", **asdict(result.agent)},
        "model": {"configuration": model_configuration, "name": model_name, "backend": backend},
        "agent_timeout_seconds": timeout_seconds,
        "timing": {
            "agent_duration_seconds": result.agent.duration_seconds,
            "model_duration_seconds": result.agent.telemetry.model_duration_seconds,
            "total_duration_seconds": result.total_duration_seconds,
        },
        "workspace": str(result.workspace),
        "log_path": str(result.workspace / AGENT_LOG),
        "validation": _validation_metadata(result.validation),
        "outcome": result.outcome,
    }
    (result.workspace / RUN_METADATA).write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


def _validation_metadata(result: ValidationResult) -> dict[str, object]:
    return {
        "outcome": result.outcome,
        "build": _command_metadata(result.build),
        "visible": _command_metadata(result.visible),
        "hidden": _command_metadata(result.hidden),
    }


def _command_metadata(result: CommandResult | None) -> dict[str, object] | None:
    if result is None:
        return None
    return asdict(result)
