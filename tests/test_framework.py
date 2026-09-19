from __future__ import annotations

import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from hasebench.agents import (
    AgentRunRequest,
    AgentRunResult,
    OpenCodeAgentRunner,
    _agent_environment,
    _default_opencode_executable,
    _terminate_process_tree,
    extract_opencode_telemetry,
)
from hasebench.cli import _compact_complexity, _print_result, _print_run_result, _run_all, _validate_all, main
from hasebench.models import Task
from hasebench.runs import AGENT_LOG, AUTONOMOUS_INSTRUCTION, RUN_METADATA, AutonomousRunResult, run_autonomous
from hasebench.reports import RunSummaryRow, write_markdown_summary
from hasebench.tasks import discover_tasks, find_task
from hasebench.validation import (
    CommandResult,
    ValidationResult,
    _cmake_debug_flags,
    _deduplicate_environment,
    validate_rust,
    validate_task,
)
from hasebench.workspaces import (
    MSVC_RUNTIME_POLICY,
    WORKSPACE_METADATA,
    WorkspaceCandidate,
    discover_workspaces,
    prepare_workspace,
    task_for_workspace,
)


class FrameworkTests(unittest.TestCase):
    def test_autonomous_run_rejects_a_bare_server_model_path_before_launch(self) -> None:
        errors = StringIO()
        with patch("sys.argv", ["hasebench", "run", "cpp_001", "--agent", "opencode",
                                "--model", "/home/ape/models/hf/Qwen3.8-27B-FP8"]), \
             redirect_stderr(errors), patch("hasebench.cli.run_autonomous") as launch:
            with self.assertRaises(SystemExit) as exit_result:
                main()
        self.assertEqual(exit_result.exception.code, 2)
        self.assertIn("provider/model selector", errors.getvalue())
        launch.assert_not_called()

    def test_autonomous_prompt_limits_temporary_files_to_workspace_tmp(self) -> None:
        self.assertIn("`TMP` and `TEMP`", AUTONOMOUS_INSTRUCTION)
        self.assertIn("`.hasebench-tmp`", AUTONOMOUS_INSTRUCTION)
        self.assertIn("temporary\nfiles or directories elsewhere", AUTONOMOUS_INSTRUCTION)

    def test_cpp_tasks_are_discovered(self) -> None:
        cpp_tasks = [task.identifier for task in discover_tasks() if task.language == "cpp"]
        self.assertEqual(cpp_tasks, [f"cpp_{index:03}" for index in range(1, 22)])
        self.assertEqual(find_task("cpp_003").standard, "c++20")

    def test_rust_tasks_are_discovered(self) -> None:
        self.assertEqual(
            [task.identifier for task in discover_tasks() if task.language == "rust"],
            ["rust_001", "rust_002"],
        )
        self.assertEqual(find_task("rust_001").standard, "Rust 2024")
        self.assertEqual(find_task("rust_002").standard, "Rust 2024")

    def test_workspace_contains_no_hidden_validator(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = prepare_workspace(find_task("cpp_001"), Path(temporary))
            self.assertTrue((workspace / "TASK.md").is_file())
            self.assertTrue((workspace / "CMakeLists.txt").is_file())
            self.assertTrue((workspace / ".git").is_dir())
            self.assertTrue((workspace / ".hasebench-msvc-runtime.hpp").is_file())
            self.assertEqual(
                (workspace / "AGENTS.md").read_text(encoding="utf-8"),
                "# Workspace instructions\n\n"
                "This directory is a standalone software project. Follow the task specification\n"
                "in `TASK.md`, keep changes within this project, and use its build and test\n"
                "commands as needed.\n",
            )
            self.assertTrue((workspace / WORKSPACE_METADATA).is_file())
            self.assertFalse((workspace / "validator").exists())
            self.assertFalse((workspace / "hidden_tests.cpp").exists())
            self.assertEqual(task_for_workspace(workspace), "cpp_001")
            self.assertIn('"workspace_id"', (workspace / WORKSPACE_METADATA).read_text(encoding="utf-8"))

    def test_rust_workspace_omits_cpp_runtime_policy_and_hidden_validator(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = prepare_workspace(find_task("rust_001"), Path(temporary))
            self.assertTrue((workspace / "Cargo.toml").is_file())
            self.assertTrue((workspace / "Cargo.lock").is_file())
            self.assertFalse((workspace / MSVC_RUNTIME_POLICY).exists())
            self.assertFalse((workspace / "validator").exists())
            self.assertEqual(task_for_workspace(workspace), "rust_001")

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

    def test_msvc_assertion_flags_are_platform_specific(self) -> None:
        flags = _cmake_debug_flags()
        if __import__("os").name == "nt":
            self.assertIn("/FI", flags)
            self.assertIn("msvc_noninteractive_assert.hpp", flags)
            self.assertTrue(flags.endswith("msvc_noninteractive_assert.hpp\""))
        else:
            self.assertIsNone(flags)

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
            _print_result(result, "Expression evaluator", "M", False)
        self.assertIn("Task:       cpp_001 - Expression evaluator\nComplexity: M", output.getvalue())

    def test_batch_validation_report_includes_task_complexity(self) -> None:
        command = CommandResult(0, "", 0.0)
        result = ValidationResult("cpp_001", command, command, command)
        candidate = WorkspaceCandidate(Path("work/run-A"), "cpp_001", "run-A")
        output = StringIO()
        with patch("hasebench.cli.discover_workspaces", return_value=[candidate]), \
             patch("hasebench.cli.validate_task", return_value=result), \
             redirect_stdout(output):
            self.assertEqual(_validate_all(None, False), 0)
        self.assertIn("run-A / cpp_001 - Expression evaluator (M): SUCCESS", output.getvalue())

    def test_complexity_labels_are_compact(self) -> None:
        self.assertEqual(_compact_complexity("easy"), "E")
        self.assertEqual(_compact_complexity("medium"), "M")
        self.assertEqual(_compact_complexity("hard"), "H")
        self.assertEqual(_compact_complexity("very hard"), "VH")

    def test_validation_dispatches_cpp_tasks_to_the_cpp_validator(self) -> None:
        task = find_task("cpp_001")
        expected = object()
        with patch("hasebench.validation.validate_cpp", return_value=expected) as validate_cpp:
            result = validate_task(Path("workspace"), task)
        self.assertIs(result, expected)
        validate_cpp.assert_called_once_with(Path("workspace"), task)

    def test_validation_rejects_an_unregistered_language(self) -> None:
        task = Task("python_001", "Python task", "python", "3.12", "easy", 1, Path("task"))
        with self.assertRaisesRegex(ValueError, "No validator is available for python"):
            validate_task(Path("workspace"), task)

    def test_rust_validation_builds_and_runs_visible_and_external_hidden_tests(self) -> None:
        task = Task("rust_001", "Rust task", "rust", "rust-2024", "easy", 1, Path("task"))
        success = CommandResult(0, "", 0.0)
        workspace = Path("workspace")
        with patch("hasebench.validation._run", return_value=success) as run:
            result = validate_rust(workspace, task)

        self.assertEqual(result.outcome, "SUCCESS")
        self.assertEqual(run.call_count, 5)
        self.assertEqual(run.call_args_list[0].args, (
            [
                "cargo", "build", "--locked", "--manifest-path", str(workspace / "Cargo.toml"),
                "--target-dir", str(workspace / "build" / "hasebench"),
            ],
            workspace,
            120,
        ))
        self.assertEqual(run.call_args_list[1].args[0][-1], "--no-run")
        hidden_arguments = run.call_args_list[4].args[0]
        self.assertEqual(hidden_arguments[:6], [
            "cargo", "test", "--locked", "--manifest-path", str(task.root / "validator" / "Cargo.toml"),
            "--target-dir",
        ])
        self.assertEqual(hidden_arguments[6], str(workspace / "build" / "hasebench-hidden"))
        self.assertEqual(hidden_arguments[7], "--config")
        self.assertTrue(hidden_arguments[8].startswith("patch.crates-io.rust_001.path="))
        self.assertEqual(run.call_args_list[3].args[0], [*hidden_arguments, "--no-run"])

    def test_rust_validation_stops_after_a_build_failure(self) -> None:
        task = Task("rust_001", "Rust task", "rust", "rust-2024", "easy", 1, Path("task"))
        failure = CommandResult(1, "compiler error", 0.0)
        with patch("hasebench.validation._run", return_value=failure) as run:
            result = validate_rust(Path("workspace"), task)
        self.assertEqual(result.outcome, "COMPILATION_FAILURE")
        self.assertEqual(run.call_count, 1)

    def test_rust_validation_classifies_hidden_test_compilation_failure(self) -> None:
        task = Task("rust_001", "Rust task", "rust", "Rust 2024", "medium", 1, Path("task"))
        success = CommandResult(0, "", 0.0)
        failure = CommandResult(1, "type error", 0.0)
        with patch("hasebench.validation._run", side_effect=[success, success, success, failure]) as run:
            result = validate_rust(Path("workspace"), task)
        self.assertEqual(result.outcome, "COMPILATION_FAILURE")
        self.assertIs(result.visible, success)
        self.assertIsNone(result.hidden)
        self.assertEqual(run.call_count, 4)

    def test_opencode_runner_uses_non_interactive_workspace_command(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            request = AgentRunRequest(workspace, "solve it", "hase/qwen", 30, workspace / "agent.log", "xhigh")
            process = type("Process", (), {
                "returncode": 0,
                "communicate": lambda self, timeout: ('{"type":"text"}\n', None),
            })()
            with patch("hasebench.agents.subprocess.Popen", return_value=process) as popen:
                result = OpenCodeAgentRunner("opencode-test").run(request)
            self.assertEqual(result.outcome, "SUCCESS")
            self.assertEqual(popen.call_args.args[0], [
                "opencode-test", "run", "--dir", str(workspace), "--model", "hase/qwen", "--variant", "xhigh",
                "--format", "json", "--auto", "solve it",
            ])
            self.assertEqual(popen.call_args.kwargs["env"]["TEMP"], str(workspace / ".hasebench-tmp"))
            self.assertEqual(popen.call_args.kwargs["encoding"], "utf-8")
            self.assertEqual(popen.call_args.kwargs["errors"], "replace")
            self.assertEqual(request.log_path.read_text(encoding="utf-8"), '{"type":"text"}\n')

    def test_windows_timeout_terminates_the_complete_process_tree(self) -> None:
        process = type("Process", (), {"pid": 4321})()
        with patch("hasebench.agents.os.name", "nt"), \
             patch("hasebench.agents.subprocess.run") as run:
            _terminate_process_tree(process)
        self.assertEqual(run.call_args.args[0], ["taskkill", "/PID", "4321", "/T", "/F"])

    def test_agent_environment_contains_workspace_temp_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            environment = _agent_environment(workspace)
            self.assertEqual(environment["TMP"], str(workspace / ".hasebench-tmp"))
            self.assertTrue((workspace / ".hasebench-tmp").is_dir())
            self.assertEqual(environment["OPENCODE_DISABLE_PROJECT_CONFIG"], "1")
            self.assertIn('/FI"', environment["CL"])

    def test_opencode_telemetry_uses_json_stream_usage_and_excludes_tool_time(self) -> None:
        output = "\n".join([
            '{"type":"step_start","timestamp":1000,"part":{"messageID":"m"}}',
            '{"type":"tool_use","part":{"messageID":"m","time":{"start":1500,"end":2500}}}',
            '{"type":"step_finish","timestamp":5000,"part":{"messageID":"m","tokens":{"total":2048,"output":120}}}',
            '{"type":"step_start","timestamp":6000,"part":{"messageID":"n"}}',
            '{"type":"step_finish","timestamp":8000,"part":{"messageID":"n","tokens":{"total":4096,"output":80}}}',
            'not JSON',
        ])
        telemetry = extract_opencode_telemetry(output)
        self.assertEqual(telemetry.max_context_tokens, 4096)
        self.assertEqual(telemetry.generated_tokens, 200)
        self.assertEqual(telemetry.model_duration_seconds, 5.0)
        self.assertEqual(telemetry.generation_tokens_per_second, 40.0)

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
                 patch("hasebench.runs.validate_task", return_value=validation):
                result = run_autonomous(
                    find_task("cpp_001"), FakeRunner(), "hase/qwen", "Qwen 27B", "llama.cpp", 900, "A3B", "xhigh"
                )
            metadata = (workspace / RUN_METADATA).read_text(encoding="utf-8")
            self.assertEqual(result.outcome, "SUCCESS")
            self.assertEqual((workspace / AGENT_LOG).read_text(encoding="utf-8"), "agent output")
            self.assertIn('"mode": "autonomous"', metadata)
            self.assertIn('"configuration": "hase/qwen"', metadata)
            self.assertIn('"title": "Expression evaluator"', metadata)
            self.assertIn('"backend": "llama.cpp"', metadata)
            self.assertIn('"variant": "xhigh"', metadata)
            self.assertIn('"total_duration_seconds"', metadata)
            self.assertIn('"outcome": "SUCCESS"', metadata)

    def test_run_all_uses_each_discovered_task_and_continues_after_failure(self) -> None:
        arguments = type("Arguments", (), {
            "task_filter": None, "agent": "opencode", "model": "test", "backend": "test", "variant": None,
        })()
        row = object()
        with patch("hasebench.cli._run_one", side_effect=[(0, row), (1, row)] + [(0, row)] * 21) as run_one, \
             patch("hasebench.cli.write_markdown_summary"), redirect_stdout(StringIO()) as output:
            self.assertEqual(_run_all(None, arguments), 1)
        self.assertIn("Summary:", output.getvalue())
        self.assertEqual(
            [call.args[0].identifier for call in run_one.call_args_list],
            [f"cpp_{index:03}" for index in range(1, 22)] + ["rust_001", "rust_002"],
        )

    def test_markdown_summary_contains_a_result_table(self) -> None:
        row = RunSummaryRow(
            "cpp_001", "Expression evaluator", "M", "PASS", "PASS", "PASS", "PASS", 4096, 200, 40.0, 5.0, 8.0, 10.0,
            "SUCCESS", Path("work/run-A"),
        )
        with tempfile.TemporaryDirectory() as temporary:
            with patch("hasebench.reports.repository_root", return_value=Path(temporary)):
                report = write_markdown_summary([row], "opencode", "hase/qwen", "llama.cpp")
            content = report.read_text(encoding="utf-8")
        self.assertIn("| Task | Description | Complexity | Agent | Build | Visible | Hidden | Context | Generation |", content)
        self.assertIn("| cpp_001 | Expression evaluator | M | PASS | PASS | PASS | PASS | 4,096 | 200 @ 40.00 tok/s |", content)

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
            _print_run_result(result, "Expression evaluator", "medium", "hase/qwen", None, False)
        self.assertIn("Build:      PASS", output.getvalue())
        self.assertIn("Visible:    PASS", output.getvalue())
        self.assertIn("Hidden:     PASS", output.getvalue())
        self.assertIn("Context:    unavailable", output.getvalue())
        self.assertIn("Full time:", output.getvalue())
        self.assertIn("Result:     SUCCESS", output.getvalue())
        self.assertIn("Task:       cpp_001 - Expression evaluator", output.getvalue())


if __name__ == "__main__":
    unittest.main()
