# Hase Bench

The current milestone supports manual and autonomous C++ benchmark runs through OpenCode.

## Python environment

Hase Bench requires exactly Python 3.12.4. Create or reconcile the repository-local environment before using the framework:

```powershell
.\tools\setup-env.ps1
.\.venv\Scripts\Activate.ps1

python -m hasebench list
python -m pytest
```

The bootstrapper creates `.venv`, installs the project in editable mode, and pins its packaging and test dependencies. It rejects any interpreter other than Python 3.12.4. Run it again at any time to restore the declared package versions.

To completely recreate the environment:

```powershell
.\tools\setup-env.ps1 -Recreate
```

Run the commands from a Visual Studio 2026 x64 Developer PowerShell (or an
equivalently initialized MSVC environment). `cl.exe` and CMake must be on
`PATH`; activate `.venv` first. The framework deliberately does not select a GCC or Clang fallback.

From the repository root:

```powershell
python -m hasebench list
python -m hasebench info cpp_001
python -m hasebench prepare cpp_001
python -m hasebench prepare cpp_001 --label A3B
```

Enter the printed workspace, launch OpenCode yourself, and ask it to read `TASK.md`. When finished, validate independently:

```powershell
python -m hasebench validate <workspace>
python -m hasebench validate --all
python -m hasebench validate --all --task cpp_001
```

Use `--verbose` to print captured CMake and CTest diagnostics. Preparation copies only `starter/` and `TASK.md`; hidden validators remain under canonical `tasks/` and are never placed in the workspace.

`validate --all` scans only direct children of `work/` containing Hase Bench workspace metadata. It continues past malformed workspaces and prints a concise per-run and aggregate result summary, including task complexity (`E`, `M`, `H`, or `VH`).

Debug C++ validation keeps assertions enabled but configures the MSVC CRT and
Windows error handling for non-interactive execution. An assertion is reported
in the captured CTest output and the test process exits; validation never waits
for an Abort/Retry/Ignore dialog.

## Autonomous OpenCode runs

Run one task in a fresh autonomous workspace:

```powershell
python -m hasebench run cpp_001 --agent opencode --model hase/qwen27b-q4 --label 27BQ4
python -m hasebench run cpp_001 --agent opencode --model llama-hase/qwen3.8-27b --backend llama.cpp --variant xhigh --label 27B-xhigh
```

Run every available task, or limit an all-task run to a task ID:

```powershell
python -m hasebench run --all --agent opencode --model hase/qwen27b-q4 --label 27BQ4
python -m hasebench run --all --task cpp_003 --agent opencode --model hase/qwen27b-q4
```

| Parameter | Required | Purpose | Example |
| --- | --- | --- | --- |
| `TASK` or `--all` | Yes | Run one task, or every available task. | `cpp_001`, `--all` |
| `--task` | With `--all`, optional | Restrict a batch to one task ID. | `--task cpp_003` |
| `--agent` | Yes | Coding agent to launch. | `--agent opencode` |
| `--model` | Yes | OpenCode provider/model selector, passed unchanged to OpenCode. | `--model llama-hase/qwen3.8-27b` |
| `--variant` | No | Provider-specific reasoning effort, passed as OpenCode's `--variant`. | `--variant medium`, `--variant xhigh` |
| `--model-name` | No | Descriptive model name retained in metadata; defaults to `--model`. | `--model-name Qwen-3.8-27B` |
| `--backend` | No | Backend label retained in metadata and reports. | `--backend llama.cpp` |
| `--label` | No | Safe suffix for each fresh workspace name. | `--label 27B-xhigh` |
| `--timeout` | No | Agent time limit in seconds; default is 1,800 (30 minutes). | `--timeout 1800` |
| `--verbose` | No | Print OpenCode, compiler, and test diagnostics. | `--verbose` |

Configure the provider, endpoint, credentials, and any model alias in OpenCode; the benchmark does not hard-code hase connection settings. The selected variant, model/backend labels, and timing are retained in run metadata and reports.

Each autonomous run creates a new `_aut_opencode` workspace, runs OpenCode with non-interactive JSON output and permission auto-approval inside that workspace, then validates it with the same visible and hidden CMake validators as manual runs. `TEMP` and `TMP` are redirected into the workspace so ordinary agent-created temporary files are preserved there too. The workspace is always preserved. It contains `hasebench-agent.log` and `hasebench-run.json`, recording the selected configuration, model/backend labels, agent exit status/timing, validation commands/results, and final classification. Use `--verbose` to print captured agent and validation diagnostics.

After every autonomous task, the console prints its workspace, model, build, visible-test, hidden-test, and final result. It also prints OpenCode's maximum observed context use, generated-token rate, estimated model time, agent elapsed time, and full benchmark elapsed time. The model time is an estimate derived from OpenCode step timing with recorded tool time removed; it is the suitable initial value for an electricity-time estimate, while full time includes workspace preparation and authoritative validation. If an OpenCode version does not emit JSON telemetry, those fields are explicitly shown as unavailable. Every command also writes a Markdown summary table under `results/`; `run --all` produces one aggregate table for the batch, while a single-task run produces a one-row table. These generated reports are ignored by Git.

Currently available tasks are CPP-001 (expression evaluator), CPP-002 (CSV parser), CPP-003 (generic LRU cache), CPP-004 (thread pool), CPP-005 (SPSC ring buffer), CPP-006 (binary serialization), and CPP-007 (template lifetime repair). See [progress.md](progress.md) for the current stage.
See [TASKS.md](TASKS.md) for the maintained implemented/planned C++ task catalogue and difficulty rationale.

Prompt example:
```markdown
Read TASK.md and solve the task.

The current directory is the entire project scope for this task. Do not inspect, list, read, or modify parent or ancestor directories, even if they are visible through Git. Do not change files outside the current workspace.

You may inspect this workspace, edit the implementation, configure/build the project, and run the visible tests.

Continue working until you believe the task is complete.

Do not ask me for implementation guidance unless you are genuinely blocked.
```
