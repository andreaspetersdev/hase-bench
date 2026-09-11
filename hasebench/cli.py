from __future__ import annotations

import argparse
import sys
from pathlib import Path

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
    validate = commands.add_parser("validate", help="independently validate one or all workspaces")
    validate_target = validate.add_mutually_exclusive_group(required=True)
    validate_target.add_argument("workspace", nargs="?", type=Path)
    validate_target.add_argument("--all", action="store_true", help="validate marked workspaces directly under work/")
    validate.add_argument("--task", help="limit --all to one task ID")
    validate.add_argument("--verbose", action="store_true", help="print CMake and test output")
    return parser


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
            workspace = prepare_workspace(find_task(args.task))
            print(f"Workspace:\n{workspace}")
            return 0
        if args.all:
            return _validate_all(args.task, args.verbose)
        return _validate_one(args.workspace.resolve(), args.verbose)
    except (KeyError, ValueError, OSError) as error:
        print(f"error: {error}")
        return 2


def _validate_one(workspace: Path, verbose: bool) -> int:
    task = find_task(task_for_workspace(workspace))
    if task.language != "cpp":
        raise ValueError(f"No validator is available for {task.language}")
    result = validate_cpp(workspace, task)
    _print_result(result, verbose)
    return 0 if result.outcome == "SUCCESS" else 1


def _validate_all(task_filter: str | None, verbose: bool) -> int:
    normalized_filter = task_filter.lower() if task_filter else None
    candidates = discover_workspaces()
    if normalized_filter:
        candidates = [candidate for candidate in candidates if candidate.task_id == normalized_filter]

    passed = 0
    rows: list[tuple[str, str, str, str, str]] = []
    for candidate in candidates:
        task_id = candidate.task_id or "unknown"
        workspace_id = candidate.workspace_id or candidate.path.name
        if candidate.metadata_error:
            outcome = "WORKSPACE_ERROR"
            visible = hidden = "NOT_RUN"
            detail = candidate.metadata_error
        else:
            try:
                task = find_task(task_id)
                if task.language != "cpp":
                    raise ValueError(f"No validator is available for {task.language}")
                result = validate_cpp(candidate.path, task)
                outcome = result.outcome
                visible = _status(result.visible)
                hidden = _status(result.hidden)
                detail = ""
                if verbose:
                    _print_result(result, True)
            except (KeyError, ValueError, OSError) as error:
                outcome = "WORKSPACE_ERROR"
                visible = hidden = "NOT_RUN"
                detail = str(error)
        rows.append((task_id, workspace_id, outcome, visible, hidden, detail))
        passed += outcome == "SUCCESS"
        suffix = f"; {detail}" if detail else ""
        print(f"{workspace_id} / {task_id}: {_colored_outcome(outcome)} "
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


def _print_result(result: object, verbose: bool) -> None:
    print(f"Task:       {result.task}\nBuild:      {'PASS' if result.build.returncode == 0 else 'FAIL'}")
    if result.visible is not None:
        print(f"Visible:    {_status(result.visible)}")
    if result.hidden is not None:
        print(f"Hidden:     {_status(result.hidden)}")
    print(f"Result:     {_colored_outcome(result.outcome)}")
    if verbose:
        for name, command in (("Build", result.build), ("Visible", result.visible), ("Hidden", result.hidden)):
            if command is not None:
                print(f"\n--- {name} ({command.duration_seconds:.2f}s) ---\n{command.output}")
