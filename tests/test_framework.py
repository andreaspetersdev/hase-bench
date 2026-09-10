from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from hasebench.tasks import discover_tasks, find_task
from hasebench.validation import _deduplicate_environment
from hasebench.workspaces import WORKSPACE_METADATA, prepare_workspace, task_for_workspace


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

    def test_preparations_are_never_reused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertNotEqual(prepare_workspace(find_task("cpp_003"), root), prepare_workspace(find_task("cpp_003"), root))

    def test_child_environment_deduplicates_case_insensitive_names(self) -> None:
        environment = _deduplicate_environment({"PATH": "first", "Path": "second", "HOME": "home"})
        self.assertEqual(environment, {"PATH": "first", "HOME": "home"})


if __name__ == "__main__":
    unittest.main()
