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
            self.assertIn("cpp_001 v2 | AGENT_TIMEOUT | SUCCESS", table)
            exported = list(csv.DictReader(StringIO(render_csv(data))))
            self.assertEqual(len(exported), 4)
            self.assertEqual({row["run_id"] for row in exported if row["selected"] == "True"},
                             {"older", "latest", "other"})

    @staticmethod
    def _write(root: Path, name: str, timestamp: str, model: str, version: int, outcome: str) -> None:
        workspace = root / name
        workspace.mkdir()
        (workspace / RUN_METADATA).write_text(json.dumps({
            "schema_version": 1, "run_id": name, "timestamp": timestamp, "mode": "autonomous",
            "task": {"id": "cpp_001", "version": version, "title": "Expression evaluator"},
            "agent": {"name": "opencode"},
            "model": {"configuration": model, "backend": "llama.cpp", "variant": "medium"},
            "validation": {"outcome": outcome}, "outcome": outcome,
            "timing": {"agent_duration_seconds": 1.0, "model_duration_seconds": None,
                       "total_duration_seconds": 2.0},
        }), encoding="utf-8")
