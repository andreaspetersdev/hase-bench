from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .agents import OpenCodeAgentRunner
from .reports import RunSummaryRow, summary_row, write_markdown_summary
from .runs import AGENT_LOG, RUN_METADATA, AutonomousRunResult, run_autonomous
from .tasks import discover_tasks, find_task
from .validation import validate_cpp
from .workspaces import discover_workspaces, prepare_workspace, task_for_workspace


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python -m hasebench")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="list available benchmark tasks")
    info = commands.add_parser("info", help="show task metadata")
    info.add_argument("task")
    prepare = commands.add_parser("prepare", help="create a clean manual workspace")
    prepare.add_argument("task")
    prepare.add_argument("--label", help="optional model or run label included in the workspace name")
    run = commands.add_parser("run", help="create a workspace, run an agent, and validate it")
    run_target = run.add_mutually_exclusive_group(required=True)
    run_target.add_argument("task", nargs="?")
    run_target.add_argument("--all", action="store_true", help="run every available task")
    run.add_argument("--task", dest="task_filter", help="limit run --all to one task ID")
    run.add_argument("--agent", choices=["opencode"], required=True, help="coding agent to execute")
    run.add_argument("--model", required=True, help="OpenCode model or configuration selector")
    run.add_argument("--model-name", help="model name retained in run metadata (defaults to --model)")
    run.add_argument("--backend", default="opencode-managed", help="backend retained in run metadata")
    run.add_argument("--variant", help="OpenCode reasoning-effort variant, for example medium or xhigh")
    run.add_argument("--timeout", type=_positive_int, default=1800, help="agent timeout in seconds (default: 1800)")
    run.add_argument("--label", help="optional safe label included in each workspace name")
    run.add_argument("--verbose", action="store_true", help="print captured agent and validation diagnostics")
    validate = commands.add_parser("validate", help="independently validate one or all workspaces")
    validate_target = validate.add_mutually_exclusive_group(required=True)
    validate_target.add_argument("workspace", nargs="?", type=Path)
    validate_target.add_argument("--all", action="store_true", help="validate marked workspaces directly under work/")
    validate.add_argument("--task", help="limit --all to one task ID")
    validate.add_argument("--verbose", action="store_true", help="print CMake and test output")
    return parser


def _positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return parsed


def main() -> int:
    parser = _parser()
    args = parser.parse_args()
    if args.command == "validate":
        if args.all and args.workspace is not None:
            parser.error("validate accepts either WORKSPACE or --all, not both")
        if not args.all and args.workspace is None:
            parser.error("validate requires WORKSPACE or --all")
        if args.task and not args.all:
            parser.error("--task is only available with validate --all")
    if args.command == "run" and args.task_filter and not args.all:
        parser.error("--task is only available with run --all")
    try:
        if args.command == "list":
            for task in discover_tasks():
                print(f"{task.identifier:<8} {task.language:<4} {task.difficulty:<10} {task.title}")
            return 0
        if args.command == "info":
            task = find_task(args.task)
            print(f"ID:         {task.identifier}\nTitle:      {task.title}\nLanguage:   {task.language}\n"
                  f"Standard:   {task.standard}\nDifficulty: {task.difficulty}\nVersion:    {task.version}")
            return 0
        if args.command == "prepare":
            workspace = prepare_workspace(find_task(args.task), label=args.label)
            print(f"Workspace:\n{workspace}")
            return 0
        if args.command == "run":
            if args.all:
                return _run_all(args.task_filter, args)
            exit_code, _ = _run_one(find_task(args.task), args)
            return exit_code
        if args.all:
            return _validate_all(args.task, args.verbose)
        return _validate_one(args.workspace.resolve(), args.verbose)
    except (KeyError, ValueError, OSError) as error:
        print(f"error: {error}")
        return 2


def _run_one(task: object, args: argparse.Namespace, write_summary: bool = True) -> tuple[int, RunSummaryRow]:
    if task.language != "cpp":
        raise ValueError(f"No autonomous runner is available for {task.language}")
    result = run_autonomous(
        task,
        OpenCodeAgentRunner(),
        args.model,
        args.model_name or args.model,
        args.backend,
        args.timeout,
        args.label,
        args.variant,
    )
    _print_run_result(result, task.title, task.difficulty, args.model, args.variant, args.verbose)
    row = summary_row(result, task.title, _compact_complexity(task.difficulty))
    if write_summary:
        path = write_markdown_summary([row], args.agent, args.model, args.backend, args.variant)
        print(f"Summary:   {path}")
    return (0 if result.outcome == "SUCCESS" else 1), row


def _run_all(task_filter: str | None, args: argparse.Namespace) -> int:
    selected = discover_tasks()
    if task_filter:
        normalized = task_filter.lower()
        selected = [task for task in selected if task.identifier == normalized]
        if not selected:
            raise KeyError(f"Unknown task: {task_filter}")

    failures = 0
    rows: list[RunSummaryRow] = []
    print(f"Running: {len(selected)}")
    for task in selected:
        try:
            exit_code, row = _run_one(task, args, write_summary=False)
            failures += exit_code != 0
            rows.append(row)
        except (ValueError, OSError) as error:
            failures += 1
            print(f"{task.identifier}: {_colored_outcome('RUN_ERROR')}; {error}")
    path = write_markdown_summary(rows, args.agent, args.model, args.backend, args.variant)
    print(f"\nCompleted: {len(selected)}\nPASS: {len(selected) - failures}\nFAIL: {failures}\nSummary:   {path}")
    return 0 if failures == 0 else 1


def _print_run_result(
    result: AutonomousRunResult, title: str, difficulty: str, model: str, variant: str | None, verbose: bool
) -> None:
    validation = result.validation
    telemetry = result.agent.telemetry
    variant_line = f"\nVariant:    {variant}" if variant else ""
    print(f"Workspace:  {result.workspace}\nModel:      {model}{variant_line}\n"
          f"Agent:      {_colored_outcome(result.agent.outcome)} ({_format_duration(result.agent.duration_seconds)})\n"
          f"Context:    {_format_tokens(telemetry.max_context_tokens)}\n"
          f"Generation: {_format_generation(telemetry.generated_tokens, telemetry.generation_tokens_per_second)}\n"
          f"Model time: {_format_duration(telemetry.model_duration_seconds, estimated=True)}\n"
          f"Full time:  {_format_duration(result.total_duration_seconds)}")
    _print_result(validation, title, _compact_complexity(difficulty), verbose)
    print(f"Metadata:   {result.workspace / RUN_METADATA}\nLog:        {result.workspace / AGENT_LOG}")
    if verbose:
        print(f"\n--- Agent ({_format_duration(result.agent.duration_seconds)}) ---\n{result.agent.output}")


def _format_duration(seconds: float | None, estimated: bool = False) -> str:
    if seconds is None:
        return "unavailable"
    suffix = " (estimated)" if estimated else ""
    minutes, remainder = divmod(seconds, 60)
    return f"{int(minutes)}m {remainder:.1f}s{suffix}" if minutes else f"{remainder:.1f}s{suffix}"


def _format_tokens(tokens: int | None) -> str:
    return "unavailable" if tokens is None else f"{tokens:,} tokens"


def _format_generation(tokens: int | None, speed: float | None) -> str:
    if tokens is None or speed is None:
        return "unavailable"
    return f"{tokens:,} tokens ({speed:.2f} tok/s)"


def _validate_one(workspace: Path, verbose: bool) -> int:
    task = find_task(task_for_workspace(workspace))
    if task.language != "cpp":
        raise ValueError(f"No validator is available for {task.language}")
    result = validate_cpp(workspace, task)
    _print_result(result, task.title, _compact_complexity(task.difficulty), verbose)
    return 0 if result.outcome == "SUCCESS" else 1


def _validate_all(task_filter: str | None, verbose: bool) -> int:
    normalized_filter = task_filter.lower() if task_filter else None
    candidates = discover_workspaces()
    if normalized_filter:
        candidates = [candidate for candidate in candidates if candidate.task_id == normalized_filter]

    passed = 0
    rows: list[str] = []
    for candidate in candidates:
        task_id = candidate.task_id or "unknown"
        workspace_id = candidate.workspace_id or candidate.path.name
        if candidate.metadata_error:
            outcome = "WORKSPACE_ERROR"
            visible = hidden = "NOT_RUN"
            detail = candidate.metadata_error
            complexity = "unknown"
            title = "unknown task"
        else:
            try:
                task = find_task(task_id)
                complexity = _compact_complexity(task.difficulty)
                title = task.title
                if task.language != "cpp":
                    raise ValueError(f"No validator is available for {task.language}")
                result = validate_cpp(candidate.path, task)
                outcome = result.outcome
                visible = _status(result.visible)
                hidden = _status(result.hidden)
                detail = ""
                if verbose:
                    _print_result(result, title, complexity, True)
            except (KeyError, ValueError, OSError) as error:
                outcome = "WORKSPACE_ERROR"
                visible = hidden = "NOT_RUN"
                detail = str(error)
                complexity = "unknown"
                title = "unknown task"
        rows.append(outcome)
        passed += outcome == "SUCCESS"
        suffix = f"; {detail}" if detail else ""
        print(f"{workspace_id} / {task_id} - {title} ({complexity}): {_colored_outcome(outcome)} "
              f"(visible: {_colored_status(visible)}, hidden: {_colored_status(hidden)}{suffix})")

    failed = len(rows) - passed
    print(f"\nValidated: {len(rows)}\nPASS: {passed}\nFAIL: {failed}")
    return 0 if failed == 0 else 1


def _status(command: object) -> str:
    return "NOT_RUN" if command is None else ("PASS" if command.returncode == 0 else "FAIL")


def _colored_outcome(outcome: str) -> str:
    return _color(outcome, "32" if outcome == "SUCCESS" else "31")


def _colored_status(status: str) -> str:
    if status == "PASS":
        return _color(status, "32")
    if status == "FAIL":
        return _color(status, "31")
    return status


def _color(text: str, code: str) -> str:
    """Use ANSI colour only for an interactive terminal."""
    if not sys.stdout.isatty():
        return text
    return f"\033[{code}m{text}\033[0m"


def _compact_complexity(difficulty: str) -> str:
    labels = {
        "very easy": "VE",
        "easy": "E",
        "medium": "M",
        "hard": "H",
        "very hard": "VH",
    }
    return labels.get(difficulty.lower(), difficulty.upper())


def _print_result(result: object, title: str, complexity: str, verbose: bool) -> None:
    print(f"Task:       {result.task} - {title}\nComplexity: {complexity}\n"
          f"Build:      {'PASS' if result.build.returncode == 0 else 'FAIL'}")
    if result.visible is not None:
        print(f"Visible:    {_status(result.visible)}")
    if result.hidden is not None:
        print(f"Hidden:     {_status(result.hidden)}")
    print(f"Result:     {_colored_outcome(result.outcome)}")
    if verbose:
        for name, command in (("Build", result.build), ("Visible", result.visible), ("Hidden", result.hidden)):
            if command is not None:
                print(f"\n--- {name} ({command.duration_seconds:.2f}s) ---\n{command.output}")
