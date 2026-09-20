"""Read saved autonomous runs and compare like-for-like task versions."""

from __future__ import annotations

import csv
import io
import json
from collections.abc import Iterable
from dataclasses import asdict, dataclass
from pathlib import Path

from .runs import RUN_METADATA
from .tasks import find_task, repository_root


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
    output_token_max: int | None
    outcome: str
    validation_outcome: str
    agent_seconds: float | None
    model_seconds: float | None
    total_seconds: float | None
    context_tokens: int | None
    generated_tokens: int | None
    generation_tokens_per_second: float | None
    workspace: str

    @property
    def configuration(self) -> str:
        output = str(self.output_token_max) if self.output_token_max is not None else "default"
        return " / ".join((self.agent, self.model, self.backend, self.variant, f"output={output}"))


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
            telemetry = agent.get("telemetry") or {}
            if not isinstance(telemetry, dict):
                raise ValueError("invalid agent telemetry")
            model_seconds = _number(timing.get("model_duration_seconds"))
            generated_tokens = _integer(telemetry.get("generated_tokens"))
            runs.append(RecordedRun(
                str(raw["run_id"]), str(raw["timestamp"]), str(task["id"]), int(task["version"]),
                str(task["title"]), str(agent["name"]), str(model["configuration"]),
                str(model["backend"]), str(model.get("variant") or "default"),
                _integer(model.get("output_token_max")),
                str(raw["outcome"]), str(raw["validation"]["outcome"]),
                _number(timing.get("agent_duration_seconds")), model_seconds,
                _number(timing.get("total_duration_seconds")),
                _integer(telemetry.get("max_context_tokens")), generated_tokens,
                _generation_speed(generated_tokens, model_seconds), str(path),
            ))
        except (OSError, ValueError, TypeError, KeyError) as error:
            errors.append(f"{metadata}: {error}")
    return runs, errors


def _number(value: object) -> float | None:
    return float(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def _integer(value: object) -> int | None:
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _generation_speed(tokens: int | None, seconds: float | None) -> float | None:
    return tokens / seconds if tokens is not None and seconds is not None and seconds > 0 else None


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
        contexts = [run.context_tokens for run in group if run.context_tokens is not None]
        generated = [run.generated_tokens for run in group if run.generated_tokens is not None]
        timed_generation = [(run.generated_tokens, run.model_seconds) for run in group
                            if run.generated_tokens is not None
                            and run.model_seconds is not None and run.model_seconds > 0]
        generated_with_time = sum(tokens for tokens, _ in timed_generation)
        generation_seconds = sum(seconds for _, seconds in timed_generation)
        models.append({"configuration": name, "agent": group[0].agent, "model": group[0].model,
                       "backend": group[0].backend, "variant": group[0].variant,
                       "output_token_max": group[0].output_token_max, "tasks": len(group),
                       "successes": outcomes.get("SUCCESS", 0), "outcomes": outcomes,
                       "max_context_tokens": max(contexts) if contexts else None,
                       "generated_tokens": sum(generated) if generated else None,
                       "generation_tokens_per_second":
                           generated_with_time / generation_seconds if generation_seconds else None,
                       "telemetry_runs": len(timed_generation),
                       "model_seconds": _sum_known(run.model_seconds for run in group),
                       "agent_seconds": _sum_known(run.agent_seconds for run in group),
                       "total_seconds": _sum_known(run.total_seconds for run in group)})
    tasks = []
    for (task, version), group in sorted(task_groups.items()):
        tasks.append({"task": task, "version": version, "title": group[0].title,
                      "severity": _task_severity(task), "configurations": len(group),
                      "successes": sum(run.outcome == "SUCCESS" for run in group)})
    return {"attempts": [asdict(run) | {"configuration": run.configuration} for run in sorted(runs, key=lambda r: (r.timestamp, r.run_id))],
            "selected": [asdict(run) | {"configuration": run.configuration} for run in selected],
            "models": models, "tasks": tasks}


def _sum_known(values: Iterable[float | None]) -> float | None:
    known = [value for value in values if value is not None]
    return sum(known) if known else None


def render_table(data: dict[str, object]) -> str:
    model_ids = {row["configuration"]: f"M{index}"
                 for index, row in enumerate(data["models"], start=1)}
    model_rows = [
        (model_ids[row["configuration"]], row["agent"], row["model"], row["backend"], row["variant"],
         _format_output_limit(row["output_token_max"]))
        for row in data["models"]
    ]
    task_rows = [
        (f"{row['task']} v{row['version']}", row["severity"], row["title"])
        for row in data["tasks"]
    ]
    result_rows = [
        (f"{row['task']} v{row['task_version']}", model_ids[row["configuration"]], row["outcome"],
         _format_tokens(row["context_tokens"]), _format_tokens(row["generated_tokens"]),
         _format_speed(row["generation_tokens_per_second"]),
         _format_duration(row["total_seconds"]), _format_duration(row["model_seconds"], estimated=True))
        for row in sorted(data["selected"],
                          key=lambda item: (item["task"], item["task_version"],
                                            model_ids[item["configuration"]]))
    ]
    tables = [
        _plain_table(
            "Models", ("Model", "Agent", "Configuration", "Backend", "Variant", "Output max"), model_rows
        ),
        _plain_table("Tasks", ("Task", "Severity", "Description"), task_rows),
        _plain_table(
            "Results",
            ("Task", "Model", "Result", "Context", "Generated", "Speed", "Execution time", "Model execution"),
            result_rows,
        ),
    ]
    return "\n\n".join(tables) + "\n"


def _task_severity(task_id: str) -> str:
    try:
        task = find_task(task_id)
    except KeyError:
        return "Unknown"
    if task.language == "rust":
        return {"easy": "Low", "medium": "Medium", "hard": "High",
                "very hard": "Very high"}.get(task.difficulty, task.difficulty.title())
    return task.difficulty.replace("-", " ").title()


def _plain_table(title: str, headers: tuple[str, ...], rows: list[tuple[object, ...]]) -> str:
    text_rows = [tuple(str(cell) for cell in row) for row in rows]
    widths = [max([len(header), *(len(row[index]) for row in text_rows)])
              for index, header in enumerate(headers)]

    def line(cells: tuple[str, ...]) -> str:
        return "  ".join(cell.ljust(widths[index]) for index, cell in enumerate(cells)).rstrip()

    separator = tuple("-" * width for width in widths)
    return "\n".join((title, line(headers), line(separator), *(line(row) for row in text_rows)))


def _format_tokens(value: int | None) -> str:
    return "unavailable" if value is None else f"{value:,}"


def _format_output_limit(value: int | None) -> str:
    return "default" if value is None else f"{value:,}"


def _format_speed(value: float | None) -> str:
    return "unavailable" if value is None else f"{value:.2f} tok/s"


def _format_duration(value: float | None, estimated: bool = False) -> str:
    if value is None:
        return "unavailable"
    marker = " (est.)" if estimated else ""
    minutes, seconds = divmod(value, 60)
    duration = f"{int(minutes)}m {seconds:.1f}s" if minutes else f"{seconds:.1f}s"
    return duration + marker


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
