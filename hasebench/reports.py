from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from .runs import AutonomousRunResult
from .tasks import repository_root


@dataclass(frozen=True)
class RunSummaryRow:
    task: str
    title: str
    complexity: str
    agent: str
    build: str
    visible: str
    hidden: str
    context_tokens: int | None
    generated_tokens: int | None
    generation_tokens_per_second: float | None
    model_duration_seconds: float | None
    agent_duration_seconds: float
    total_duration_seconds: float
    outcome: str
    workspace: Path


def summary_row(result: AutonomousRunResult, title: str, complexity: str) -> RunSummaryRow:
    validation = result.validation
    return RunSummaryRow(
        validation.task, title, complexity,
        "PASS" if result.agent.outcome == "SUCCESS" else "FAIL",
        "PASS" if validation.build.returncode == 0 else "FAIL",
        _command_status(validation.visible), _command_status(validation.hidden),
        result.agent.telemetry.max_context_tokens, result.agent.telemetry.generated_tokens,
        result.agent.telemetry.generation_tokens_per_second, result.agent.telemetry.model_duration_seconds,
        result.agent.duration_seconds, result.total_duration_seconds, result.outcome, result.workspace,
    )


def write_markdown_summary(
    rows: list[RunSummaryRow], agent: str, model: str, backend: str, variant: str | None = None
) -> Path:
    root = repository_root() / "results"
    root.mkdir(exist_ok=True)
    path = root / f"{datetime.now().strftime('%Y%m%d-%H%M%S')}_autonomous_summary.md"
    lines = [
        "# Autonomous benchmark summary",
        "",
        f"- Agent: `{agent}`",
        f"- Model/configuration: `{model}`",
        f"- Backend: `{backend}`",
        f"- Variant: `{variant or 'default'}`",
        "",
        "| Task | Description | Complexity | Agent | Build | Visible | Hidden | Context | Generation | Model time | Agent time | Full time | Result | Workspace |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(
            f"| {row.task} | {row.title} | {row.complexity} | {row.agent} | {row.build} | {row.visible} | "
            f"{row.hidden} | {_tokens(row.context_tokens)} | {_generation(row.generated_tokens, row.generation_tokens_per_second)} | "
            f"{_duration(row.model_duration_seconds, estimated=True)} | {_duration(row.agent_duration_seconds)} | "
            f"{_duration(row.total_duration_seconds)} | {row.outcome} | `{row.workspace}` |"
        )
    passed = sum(row.outcome == "SUCCESS" for row in rows)
    lines.extend(["", f"**Completed:** {len(rows)}  ", f"**PASS:** {passed}  ", f"**FAIL:** {len(rows) - passed}", ""])
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def _command_status(command: object) -> str:
    return "NOT_RUN" if command is None else ("PASS" if command.returncode == 0 else "FAIL")


def _tokens(value: int | None) -> str:
    return "unavailable" if value is None else f"{value:,}"


def _generation(tokens: int | None, speed: float | None) -> str:
    if tokens is None or speed is None:
        return "unavailable"
    return f"{tokens:,} @ {speed:.2f} tok/s"


def _duration(seconds: float | None, estimated: bool = False) -> str:
    if seconds is None:
        return "unavailable"
    marker = " (est.)" if estimated else ""
    return f"{seconds:.1f}s{marker}"
