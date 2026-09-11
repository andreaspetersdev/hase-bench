from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from dataclasses import dataclass, field
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
    variant: str | None = None


@dataclass(frozen=True)
class AgentTelemetry:
    """Measurements extracted from an agent's machine-readable output.

    OpenCode does not expose server-side inference time. The duration below is
    an estimate: a step's wall time minus its recorded tool execution time.
    """

    max_context_tokens: int | None = None
    generated_tokens: int | None = None
    model_duration_seconds: float | None = None

    @property
    def generation_tokens_per_second(self) -> float | None:
        if self.generated_tokens is None or not self.model_duration_seconds:
            return None
        return self.generated_tokens / self.model_duration_seconds


@dataclass(frozen=True)
class AgentRunResult:
    started_at: str
    ended_at: str
    duration_seconds: float
    exit_code: int | None
    output: str
    outcome: str
    telemetry: AgentTelemetry = field(default_factory=AgentTelemetry)


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
        ]
        if request.variant:
            arguments.extend(["--variant", request.variant])
        arguments.extend([
            "--format",
            "json",
            "--auto",
            request.instruction,
        ])
        environment = _agent_environment(request.workspace)
        try:
            completed = subprocess.run(
                arguments,
                cwd=request.workspace,
                env=environment,
                text=True,
                encoding="utf-8",
                errors="replace",
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=request.timeout_seconds,
                check=False,
            )
            output = completed.stdout or ""
            outcome = "SUCCESS" if completed.returncode == 0 else "AGENT_NONZERO_EXIT"
            result = AgentRunResult(
                started_at, _timestamp(), time.monotonic() - started, completed.returncode, output, outcome,
                extract_opencode_telemetry(output),
            )
        except subprocess.TimeoutExpired as error:
            output = error.stdout or ""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            result = AgentRunResult(
                started_at, _timestamp(), time.monotonic() - started, None, output, "AGENT_TIMEOUT",
                extract_opencode_telemetry(output),
            )
        except OSError as error:
            result = AgentRunResult(started_at, _timestamp(), time.monotonic() - started, None, str(error), "AGENT_LAUNCH_FAILURE")

        request.log_path.write_text(result.output, encoding="utf-8")
        return result


def extract_opencode_telemetry(output: str) -> AgentTelemetry:
    """Read token usage and timing from OpenCode's JSON-lines event stream.

    Unrecognised or malformed events are ignored so logging cannot turn a run
    into a framework failure.
    """
    steps: dict[str, tuple[float, float | None]] = {}
    tool_intervals: dict[str, list[tuple[float, float]]] = {}
    max_context: int | None = None
    generated = 0
    found_generated = False
    for line in output.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict):
            continue
        part = event.get("part")
        if not isinstance(part, dict):
            continue
        message_id = part.get("messageID")
        timestamp = _milliseconds_to_seconds(event.get("timestamp"))
        event_type = event.get("type")
        if isinstance(message_id, str) and timestamp is not None:
            if event_type == "step_start":
                steps[message_id] = (timestamp, None)
            elif event_type == "step_finish" and message_id in steps:
                start, _ = steps[message_id]
                steps[message_id] = (start, timestamp)
        if event_type == "tool_use" and isinstance(message_id, str):
            event_time = part.get("time")
            if isinstance(event_time, dict):
                start = _milliseconds_to_seconds(event_time.get("start"))
                end = _milliseconds_to_seconds(event_time.get("end"))
                if start is not None and end is not None and end >= start:
                    tool_intervals.setdefault(message_id, []).append((start, end))
        if event_type == "step_finish":
            tokens = part.get("tokens")
            if not isinstance(tokens, dict):
                continue
            total = tokens.get("total")
            if isinstance(total, int):
                max_context = total if max_context is None else max(max_context, total)
            output_tokens = tokens.get("output")
            if isinstance(output_tokens, int):
                generated += output_tokens
                found_generated = True

    model_seconds = 0.0
    finished_steps = 0
    for message_id, (start, end) in steps.items():
        if end is None or end < start:
            continue
        tool_seconds = sum(
            max(0.0, min(end, tool_end) - max(start, tool_start))
            for tool_start, tool_end in tool_intervals.get(message_id, [])
        )
        model_seconds += max(0.0, end - start - tool_seconds)
        finished_steps += 1
    return AgentTelemetry(
        max_context_tokens=max_context,
        generated_tokens=generated if found_generated else None,
        model_duration_seconds=model_seconds if finished_steps else None,
    )


def _milliseconds_to_seconds(value: object) -> float | None:
    return value / 1000.0 if isinstance(value, (int, float)) else None


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
