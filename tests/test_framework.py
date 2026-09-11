from __future__ import annotations

import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from hasebench.cli import _compact_complexity, _print_result, _validate_all
from hasebench.tasks import discover_tasks, find_task
from hasebench.validation import CommandResult, ValidationResult, _deduplicate_environment
from hasebench.workspaces import (
    WORKSPACE_METADATA,
    WorkspaceCandidate,
    discover_workspaces,
    prepare_workspace,
    task_for_workspace,
)


class FrameworkTests(unittest.TestCase):
    def test_three_initial_tasks_are_discovered(self) -> None:
        self.assertEqual([task.identifier for task in discover_tasks()], ["cpp_001", "cpp_003", "cpp_005"])
        self.assertEqual(find_task("cpp_003").standard, "c++20")

    def test_workspace_contains_no_hidden_validator(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = prepare_workspace(find_task("cpp_001"), Path(temporary))
            self.assertTrue((workspace / "TASK.md").is_file())
            self.assertTrue((workspace / "CMakeLists.txt").is_file())
            self.assertTrue((workspace / WORKSPACE_METADATA).is_file())
            self.assertFalse((workspace / "validator").exists())
            self.assertFalse((workspace / "hidden_tests.cpp").exists())
            self.assertEqual(task_for_workspace(workspace), "cpp_001")
            self.assertIn('"workspace_id"', (workspace / WORKSPACE_METADATA).read_text(encoding="utf-8"))

    def test_preparations_are_never_reused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertNotEqual(prepare_workspace(find_task("cpp_003"), root), prepare_workspace(find_task("cpp_003"), root))

    def test_workspace_label_is_included_in_the_short_manual_name(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = prepare_workspace(find_task("cpp_001"), Path(temporary), "A3B")
            self.assertRegex(workspace.name, r"^\d{8}-\d{6}_cpp_001_man_A3B$")
            with self.assertRaises(ValueError):
                prepare_workspace(find_task("cpp_001"), Path(temporary), "A3B/model")

    def test_child_environment_deduplicates_case_insensitive_names(self) -> None:
        environment = _deduplicate_environment({"PATH": "first", "Path": "second", "HOME": "home"})
        self.assertEqual(environment, {"PATH": "first", "HOME": "home"})

    def test_workspace_discovery_requires_metadata_and_retains_corrupt_candidates(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prepared = prepare_workspace(find_task("cpp_001"), root)
            (root / "ordinary-directory").mkdir()
            corrupt = root / "corrupt-workspace"
            corrupt.mkdir()
            (corrupt / WORKSPACE_METADATA).write_text("not json", encoding="utf-8")

            candidates = discover_workspaces(root)
            self.assertEqual([candidate.path for candidate in candidates], [prepared, corrupt])
            self.assertEqual(candidates[0].task_id, "cpp_001")
            self.assertEqual(candidates[0].workspace_id, prepared.name)
            self.assertIsNotNone(candidates[1].metadata_error)

    def test_single_validation_report_includes_task_complexity(self) -> None:
        command = CommandResult(0, "", 0.0)
        result = ValidationResult("cpp_001", command, command, command)
        output = StringIO()
        with redirect_stdout(output):
            _print_result(result, "M", False)
        self.assertIn("Task:       cpp_001\nComplexity: M", output.getvalue())

    def test_batch_validation_report_includes_task_complexity(self) -> None:
        command = CommandResult(0, "", 0.0)
        result = ValidationResult("cpp_001", command, command, command)
        candidate = WorkspaceCandidate(Path("work/run-A"), "cpp_001", "run-A")
        output = StringIO()
        with patch("hasebench.cli.discover_workspaces", return_value=[candidate]), \
             patch("hasebench.cli.validate_cpp", return_value=result), \
             redirect_stdout(output):
            self.assertEqual(_validate_all(None, False), 0)
        self.assertIn("run-A / cpp_001 (M): SUCCESS", output.getvalue())

    def test_complexity_labels_are_compact(self) -> None:
        self.assertEqual(_compact_complexity("easy"), "E")
        self.assertEqual(_compact_complexity("medium"), "M")
        self.assertEqual(_compact_complexity("hard"), "H")
        self.assertEqual(_compact_complexity("very hard"), "VH")


if __name__ == "__main__":
    unittest.main()
