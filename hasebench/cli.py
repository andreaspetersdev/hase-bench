from __future__ import annotations

import argparse
from pathlib import Path

from .tasks import discover_tasks, find_task
from .validation import validate_cpp
from .workspaces import prepare_workspace, task_for_workspace


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python -m hasebench")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="list available benchmark tasks")
    info = commands.add_parser("info", help="show task metadata")
    info.add_argument("task")
    prepare = commands.add_parser("prepare", help="create a clean manual workspace")
    prepare.add_argument("task")
    validate = commands.add_parser("validate", help="independently validate a workspace")
    validate.add_argument("workspace", type=Path)
    validate.add_argument("--verbose", action="store_true", help="print CMake and test output")
    return parser


def main() -> int:
    args = _parser().parse_args()
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
        workspace = args.workspace.resolve()
        task = find_task(task_for_workspace(workspace))
        if task.language != "cpp":
            raise ValueError(f"No validator is available for {task.language}")
        result = validate_cpp(workspace, task)
        print(f"Task:       {result.task}\nBuild:      {'PASS' if result.build.returncode == 0 else 'FAIL'}")
        if result.visible is not None:
            print(f"Visible:    {'PASS' if result.visible.returncode == 0 else 'FAIL'}")
        if result.hidden is not None:
            print(f"Hidden:     {'PASS' if result.hidden.returncode == 0 else 'FAIL'}")
        print(f"Result:     {result.outcome}")
        if args.verbose:
            for name, command in (("Build", result.build), ("Visible", result.visible), ("Hidden", result.hidden)):
                if command is not None:
                    print(f"\n--- {name} ({command.duration_seconds:.2f}s) ---\n{command.output}")
        return 0 if result.outcome == "SUCCESS" else 1
    except (KeyError, ValueError, OSError) as error:
        print(f"error: {error}")
        return 2

