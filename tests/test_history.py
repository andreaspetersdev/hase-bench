from __future__ import annotations

import csv
import json
import tempfile
import unittest
from io import StringIO
from pathlib import Path

from hasebench.history import latest_attempts, load_runs, render_csv, render_table, report_data
from hasebench.runs import RUN_METADATA


class HistoryTests(unittest.TestCase):
    def test_saved_runs_keep_versions_and_failure_classifications_separate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write(root, "older", "2026-01-01T00:00:00Z", "qwen", 1, "SUCCESS")
            self._write(root, "failed", "2026-01-02T00:00:00Z", "qwen", 2, "HIDDEN_TEST_FAILURE")
            self._write(root, "latest", "2026-01-03T00:00:00Z", "qwen", 2, "SUCCESS")
            self._write(root, "other", "2026-01-04T00:00:00Z", "other", 2, "AGENT_TIMEOUT")
            (root / "ordinary").mkdir()
            corrupt = root / "corrupt"
            corrupt.mkdir()
            (corrupt / RUN_METADATA).write_text("{", encoding="utf-8")

            runs, errors = load_runs(root)
            self.assertEqual(len(runs), 4)
            self.assertEqual(len(errors), 1)
            self.assertEqual({run.run_id for run in latest_attempts(runs)}, {"older", "latest", "other"})
            data = report_data(runs)
            self.assertEqual([(row["version"], row["successes"], row["configurations"])
                              for row in data["tasks"]], [(1, 1, 1), (2, 1, 2)])
            self.assertEqual(sum(row["outcomes"].get("AGENT_TIMEOUT", 0) for row in data["models"]), 1)
            table = render_table(data)
            self.assertNotIn("Table ", table)
            self.assertIn("Models\n", table)
            self.assertIn("Tasks\n", table)
            self.assertIn("Results\n", table)
            self.assertIn("Task        Model  Result", table)
            self.assertIn("cpp_001 v2", table)
            self.assertIn("AGENT_TIMEOUT", table)
            self.assertIn("4,096", table)
            self.assertIn("50.00 tok/s", table)
            exported = list(csv.DictReader(StringIO(render_csv(data))))
            self.assertEqual(len(exported), 4)
            self.assertEqual({row["run_id"] for row in exported if row["selected"] == "True"},
                             {"older", "latest", "other"})
            self.assertEqual(exported[0]["context_tokens"], "4096")
            self.assertEqual(exported[0]["generated_tokens"], "200")
            self.assertEqual(exported[0]["generation_tokens_per_second"], "50.0")

    def test_saved_runs_without_telemetry_remain_reportable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write(root, "legacy", "2026-01-01T00:00:00Z", "qwen", 1, "SUCCESS",
                        include_telemetry=False)
            runs, errors = load_runs(root)
            self.assertEqual(errors, [])
            self.assertIsNone(runs[0].context_tokens)
            self.assertIsNone(runs[0].generated_tokens)
            self.assertIsNone(runs[0].generation_tokens_per_second)
            self.assertIn("unavailable", render_table(report_data(runs)))

    @staticmethod
    def _write(root: Path, name: str, timestamp: str, model: str, version: int, outcome: str,
               include_telemetry: bool = True) -> None:
        workspace = root / name
        workspace.mkdir()
        agent: dict[str, object] = {"name": "opencode"}
        if include_telemetry:
            agent["telemetry"] = {"max_context_tokens": 4096, "generated_tokens": 200}
        (workspace / RUN_METADATA).write_text(json.dumps({
            "schema_version": 1, "run_id": name, "timestamp": timestamp, "mode": "autonomous",
            "task": {"id": "cpp_001", "version": version, "title": "Expression evaluator"},
            "agent": agent,
            "model": {"configuration": model, "backend": "llama.cpp", "variant": "medium"},
            "validation": {"outcome": outcome}, "outcome": outcome,
            "timing": {"agent_duration_seconds": 1.0, "model_duration_seconds": 4.0,
                       "total_duration_seconds": 2.0},
        }), encoding="utf-8")
