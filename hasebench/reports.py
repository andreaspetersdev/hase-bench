from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from .runs import AutonomousRunResult
from .tasks import repository_root


@dataclass(frozen=True)
class RunSummaryRow:
    task: str
    complexity: str
    agent: str
    build: str
    visible: str
    hidden: str
    outcome: str
    workspace: Path


def summary_row(result: AutonomousRunResult, complexity: str) -> RunSummaryRow:
    validation = result.validation
    return RunSummaryRow(
        validation.task, complexity,
        "PASS" if result.agent.outcome == "SUCCESS" else "FAIL",
        "PASS" if validation.build.returncode == 0 else "FAIL",
        _command_status(validation.visible), _command_status(validation.hidden), result.outcome, result.workspace,
    )


def write_markdown_summary(rows: list[RunSummaryRow], agent: str, model: str, backend: str) -> Path:
    root = repository_root() / "results"
    root.mkdir(exist_ok=True)
    path = root / f"{datetime.now().strftime('%Y%m%d-%H%M%S')}_autonomous_summary.md"
    lines = [
        "# Autonomous benchmark summary",
        "",
        f"- Agent: `{agent}`",
        f"- Model/configuration: `{model}`",
        f"- Backend: `{backend}`",
        "",
        "| Task | Complexity | Agent | Build | Visible | Hidden | Result | Workspace |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(
            f"| {row.task} | {row.complexity} | {row.agent} | {row.build} | {row.visible} | "
            f"{row.hidden} | {row.outcome} | `{row.workspace}` |"
        )
    passed = sum(row.outcome == "SUCCESS" for row in rows)
    lines.extend(["", f"**Completed:** {len(rows)}  ", f"**PASS:** {passed}  ", f"**FAIL:** {len(rows) - passed}", ""])
    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def _command_status(command: object) -> str:
    return "NOT_RUN" if command is None else ("PASS" if command.returncode == 0 else "FAIL")
