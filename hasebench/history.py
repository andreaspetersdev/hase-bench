"""Read saved autonomous runs and compare like-for-like task versions."""

from __future__ import annotations

import csv
import io
import json
from dataclasses import asdict, dataclass
from pathlib import Path

from .runs import RUN_METADATA
from .tasks import repository_root


@dataclass(frozen=True)
class RecordedRun:
    run_id: str
    timestamp: str
    task: str
    task_version: int
    title: str
    agent: str
    model: str
    backend: str
    variant: str
    outcome: str
    validation_outcome: str
    agent_seconds: float | None
    model_seconds: float | None
    total_seconds: float | None
    workspace: str

    @property
    def configuration(self) -> str:
        return " / ".join((self.agent, self.model, self.backend, self.variant))


def load_runs(root: Path | None = None) -> tuple[list[RecordedRun], list[str]]:
    """Only direct workspaces with completed run metadata are report inputs."""
    work = root or repository_root() / "work"
    runs: list[RecordedRun] = []
    errors: list[str] = []
    if not work.exists():
        return runs, errors
    for path in sorted(work.iterdir()):
        metadata = path / RUN_METADATA
        if not path.is_dir() or not metadata.is_file():
            continue
        try:
            raw = json.loads(metadata.read_text(encoding="utf-8"))
            if raw["schema_version"] != 1 or raw["mode"] != "autonomous":
                raise ValueError("unsupported run metadata")
            task, agent, model = raw["task"], raw["agent"], raw["model"]
            timing = raw["timing"]
            runs.append(RecordedRun(
                str(raw["run_id"]), str(raw["timestamp"]), str(task["id"]), int(task["version"]),
                str(task["title"]), str(agent["name"]), str(model["configuration"]),
                str(model["backend"]), str(model.get("variant") or "default"),
                str(raw["outcome"]), str(raw["validation"]["outcome"]),
                _number(timing.get("agent_duration_seconds")), _number(timing.get("model_duration_seconds")),
                _number(timing.get("total_duration_seconds")), str(path),
            ))
        except (OSError, ValueError, TypeError, KeyError) as error:
            errors.append(f"{metadata}: {error}")
    return runs, errors


def _number(value: object) -> float | None:
    return float(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def latest_attempts(runs: list[RecordedRun]) -> list[RecordedRun]:
    """One attempt per configuration and task version; later timestamp wins."""
    latest: dict[tuple[str, str, int], RecordedRun] = {}
    for run in runs:
        key = (run.configuration, run.task, run.task_version)
        if key not in latest or (run.timestamp, run.run_id) > (latest[key].timestamp, latest[key].run_id):
            latest[key] = run
    return sorted(latest.values(), key=lambda run: (run.configuration, run.task, run.task_version))


def report_data(runs: list[RecordedRun]) -> dict[str, object]:
    selected = latest_attempts(runs)
    model_groups: dict[str, list[RecordedRun]] = {}
    task_groups: dict[tuple[str, int], list[RecordedRun]] = {}
    for run in selected:
        model_groups.setdefault(run.configuration, []).append(run)
        task_groups.setdefault((run.task, run.task_version), []).append(run)
    models = []
    for name, group in sorted(model_groups.items()):
        outcomes: dict[str, int] = {}
        for run in group:
            outcomes[run.outcome] = outcomes.get(run.outcome, 0) + 1
        models.append({"configuration": name, "tasks": len(group),
                       "successes": outcomes.get("SUCCESS", 0), "outcomes": outcomes})
    tasks = []
    for (task, version), group in sorted(task_groups.items()):
        tasks.append({"task": task, "version": version, "title": group[0].title,
                      "configurations": len(group), "successes": sum(run.outcome == "SUCCESS" for run in group)})
    return {"attempts": [asdict(run) | {"configuration": run.configuration} for run in sorted(runs, key=lambda r: (r.timestamp, r.run_id))],
            "selected": [asdict(run) | {"configuration": run.configuration} for run in selected],
            "models": models, "tasks": tasks}


def render_table(data: dict[str, object]) -> str:
    lines = ["Latest attempt per model configuration and task version", "",
             "Model configuration | Passed / Tasks | Failure classifications"]
    for row in data["models"]:
        failures = ", ".join(f"{name}={count}" for name, count in sorted(row["outcomes"].items()) if name != "SUCCESS")
        lines.append(f"{row['configuration']} | {row['successes']} / {row['tasks']} | {failures or '-'}")
    lines.extend(["", "Task/version | Passed / Configurations"])
    for row in data["tasks"]:
        lines.append(f"{row['task']} v{row['version']} {row['title']} | {row['successes']} / {row['configurations']}")
    configurations = [row["configuration"] for row in data["models"]]
    if configurations:
        by_task = {(row["task"], row["task_version"], row["configuration"]): row["outcome"]
                   for row in data["selected"]}
        lines.extend(["", "Task/version | " + " | ".join(configurations)])
        for row in data["tasks"]:
            cells = [by_task.get((row["task"], row["version"], configuration), "-")
                     for configuration in configurations]
            lines.append(f"{row['task']} v{row['version']} | " + " | ".join(cells))
    lines.extend(["", f"Saved attempts: {len(data['attempts'])}; selected: {len(data['selected'])}"])
    return "\n".join(lines) + "\n"


def render_csv(data: dict[str, object]) -> str:
    output = io.StringIO(newline="")
    fields = list(RecordedRun.__dataclass_fields__) + ["configuration", "selected"]
    writer = csv.DictWriter(output, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    selected_ids = {row["run_id"] for row in data["selected"]}
    for row in data["attempts"]:
        writer.writerow(row | {"selected": row["run_id"] in selected_ids})
    return output.getvalue()


def render_json(data: dict[str, object]) -> str:
    return json.dumps(data, indent=2) + "\n"
