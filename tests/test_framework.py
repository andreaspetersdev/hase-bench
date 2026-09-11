from __future__ import annotations

import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from hasebench.agents import (
    AgentRunRequest,
    AgentRunResult,
    OpenCodeAgentRunner,
    _agent_environment,
    _default_opencode_executable,
)
from hasebench.cli import _compact_complexity, _print_result, _print_run_result, _run_all, _validate_all
from hasebench.runs import AGENT_LOG, RUN_METADATA, AutonomousRunResult, run_autonomous
from hasebench.reports import RunSummaryRow, write_markdown_summary
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
    def test_cpp_tasks_are_discovered(self) -> None:
        self.assertEqual([task.identifier for task in discover_tasks()], ["cpp_001", "cpp_002", "cpp_003", "cpp_005"])
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

    def test_autonomous_workspace_has_a_distinct_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = prepare_workspace(find_task("cpp_001"), Path(temporary), "A3B", mode="aut_opencode")
            self.assertRegex(workspace.name, r"^\d{8}-\d{6}_cpp_001_aut_opencode_A3B$")
            self.assertIn('"mode": "aut_opencode"', (workspace / WORKSPACE_METADATA).read_text(encoding="utf-8"))

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

    def test_opencode_runner_uses_non_interactive_workspace_command(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            request = AgentRunRequest(workspace, "solve it", "hase/qwen", 30, workspace / "agent.log")
            completed = type("Completed", (), {"returncode": 0, "stdout": '{"type":"text"}\n'})()
            with patch("hasebench.agents.subprocess.run", return_value=completed) as run:
                result = OpenCodeAgentRunner("opencode-test").run(request)
            self.assertEqual(result.outcome, "SUCCESS")
            self.assertEqual(run.call_args.args[0], [
                "opencode-test", "run", "--dir", str(workspace), "--model", "hase/qwen",
                "--format", "json", "--auto", "solve it",
            ])
            self.assertEqual(run.call_args.kwargs["env"]["TEMP"], str(workspace / ".hasebench-tmp"))
            self.assertEqual(run.call_args.kwargs["encoding"], "utf-8")
            self.assertEqual(run.call_args.kwargs["errors"], "replace")
            self.assertEqual(request.log_path.read_text(encoding="utf-8"), '{"type":"text"}\n')

    def test_agent_environment_contains_workspace_temp_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            environment = _agent_environment(workspace)
            self.assertEqual(environment["TMP"], str(workspace / ".hasebench-tmp"))
            self.assertTrue((workspace / ".hasebench-tmp").is_dir())

    @unittest.skipUnless(__import__("os").name == "nt", "Windows command wrapper selection")
    def test_opencode_default_uses_windows_command_wrapper(self) -> None:
        with patch("hasebench.agents.shutil.which", return_value="C:/npm/opencode.cmd"):
            self.assertEqual(_default_opencode_executable(), "C:/npm/opencode.cmd")

    def test_autonomous_run_writes_metadata_and_uses_the_shared_validator(self) -> None:
        command = CommandResult(0, "", 0.0)
        validation = ValidationResult("cpp_001", command, command, command)
        agent = AgentRunResult("2026-01-01T00:00:00Z", "2026-01-01T00:00:01Z", 1.0, 0, "agent output", "SUCCESS")

        class FakeRunner:
            def run(self, request: AgentRunRequest) -> AgentRunResult:
                request.log_path.write_text(agent.output, encoding="utf-8")
                return agent

        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary) / "autonomous-workspace"
            workspace.mkdir()
            with patch("hasebench.runs.prepare_workspace", return_value=workspace), \
                 patch("hasebench.runs.validate_cpp", return_value=validation):
                result = run_autonomous(
                    find_task("cpp_001"), FakeRunner(), "hase/qwen", "Qwen 27B", "llama.cpp", 900, "A3B"
                )
            metadata = (workspace / RUN_METADATA).read_text(encoding="utf-8")
            self.assertEqual(result.outcome, "SUCCESS")
            self.assertEqual((workspace / AGENT_LOG).read_text(encoding="utf-8"), "agent output")
            self.assertIn('"mode": "autonomous"', metadata)
            self.assertIn('"configuration": "hase/qwen"', metadata)
            self.assertIn('"backend": "llama.cpp"', metadata)
            self.assertIn('"outcome": "SUCCESS"', metadata)

    def test_run_all_uses_each_discovered_task_and_continues_after_failure(self) -> None:
        arguments = type("Arguments", (), {"task_filter": None, "agent": "opencode", "model": "test", "backend": "test"})()
        row = object()
        with patch("hasebench.cli._run_one", side_effect=[(0, row), (1, row), (0, row), (0, row)]) as run_one, \
             patch("hasebench.cli.write_markdown_summary"), redirect_stdout(StringIO()) as output:
            self.assertEqual(_run_all(None, arguments), 1)
        self.assertIn("Summary:", output.getvalue())
        self.assertEqual([call.args[0].identifier for call in run_one.call_args_list], ["cpp_001", "cpp_002", "cpp_003", "cpp_005"])

    def test_markdown_summary_contains_a_result_table(self) -> None:
        row = RunSummaryRow("cpp_001", "M", "PASS", "PASS", "PASS", "PASS", "SUCCESS", Path("work/run-A"))
        with tempfile.TemporaryDirectory() as temporary:
            with patch("hasebench.reports.repository_root", return_value=Path(temporary)):
                report = write_markdown_summary([row], "opencode", "hase/qwen", "llama.cpp")
            content = report.read_text(encoding="utf-8")
        self.assertIn("| Task | Complexity | Agent | Build | Visible | Hidden | Result | Workspace |", content)
        self.assertIn("| cpp_001 | M | PASS | PASS | PASS | PASS | SUCCESS |", content)

    def test_autonomous_screen_report_includes_validation_details(self) -> None:
        command = CommandResult(0, "", 0.0)
        result = AutonomousRunResult(
            Path("work/run-A"),
            AgentRunResult("start", "end", 2.0, 0, "", "SUCCESS"),
            ValidationResult("cpp_001", command, command, command),
            "SUCCESS",
        )
        output = StringIO()
        with redirect_stdout(output):
            _print_run_result(result, "medium", "hase/qwen", False)
        self.assertIn("Build:      PASS", output.getvalue())
        self.assertIn("Visible:    PASS", output.getvalue())
        self.assertIn("Hidden:     PASS", output.getvalue())
        self.assertIn("Result:     SUCCESS", output.getvalue())


if __name__ == "__main__":
    unittest.main()
