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

`validate --all` scans only direct children of `work/` containing Hase Bench workspace metadata. It continues past malformed workspaces and prints a concise per-run and aggregate result summary, including each task's ID, title, and compact complexity (`E`, `M`, `H`, or `VH`). Autonomous Markdown summaries record the same task descriptions.

Debug C++ validation keeps assertions enabled but configures the MSVC CRT and
Windows error handling for non-interactive execution. An assertion is reported
in the captured CTest output and the test process exits; validation never waits
for an Abort/Retry/Ignore dialog.

Rust validation requires `cargo` and `rustc` on `PATH`. Both `starter/` and
`validator/` must contain committed `Cargo.toml` and `Cargo.lock` files, and the
starter package name must equal the task ID. The validator crate declares that
package as a dependency. During authoritative validation, the framework uses a
Cargo source patch to redirect the dependency to the isolated run workspace,
so hidden test sources stay in the canonical `validator/` directory. Cargo
build artifacts are retained under the workspace's `build/` directory.

## Autonomous OpenCode runs

Run one task in a fresh autonomous workspace:

```powershell
python -m hasebench run cpp_001 --agent opencode --model hase/qwen27b-q4 --label 27BQ4
python -m hasebench run cpp_001 --agent opencode --model llama-hase/qwen3.8-27b --backend llama.cpp --variant xhigh --label 27B-xhigh
python -m hasebench run cpp_001 --agent opencode --model vllm-hase//home/ape/models/hf/Qwen3.8-27B-FP8 --backend vllm --variant medium --label 27BFP8
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
`--model` must be OpenCode's `provider/model` selector. A server-side path such as `/home/ape/models/hf/Qwen3.8-27B-FP8` is only the model ID; for the configured `vllm-hase` provider, OpenCode lists the full selector as `vllm-hase//home/ape/models/hf/Qwen3.8-27B-FP8`. Check `opencode models vllm-hase` after changing the provider configuration.

Each autonomous run creates a new `_aut_opencode` workspace, runs OpenCode with non-interactive JSON output and permission auto-approval inside that workspace, then validates it with the same visible and hidden CMake validators as manual runs. `TEMP` and `TMP` are redirected into the workspace so ordinary agent-created temporary files are preserved there too. The workspace is always preserved. It contains `hasebench-agent.log` and `hasebench-run.json`, recording the selected configuration, model/backend labels, agent exit status/timing, validation commands/results, and final classification. Use `--verbose` to print captured agent and validation diagnostics.

After every autonomous task, the console prints its task ID and title, workspace, model, build, visible-test, hidden-test, and final result. It also prints OpenCode's maximum observed context use, generated-token rate, estimated model time, agent elapsed time, and full benchmark elapsed time. The model time is an estimate derived from OpenCode step timing with recorded tool time removed; it is the suitable initial value for an electricity-time estimate, while full time includes workspace preparation and authoritative validation. If an OpenCode version does not emit JSON telemetry, those fields are explicitly shown as unavailable. Every command also writes a Markdown summary table under `results/`; `run --all` produces one aggregate table for the batch, while a single-task run produces a one-row table. The Markdown table and per-run JSON metadata both retain the task title. These generated reports are ignored by Git.

## Compare saved runs

```powershell
python -m hasebench report
python -m hasebench report --task cpp_021
python -m hasebench report --model llama-hase/qwen3.8-27b --format csv --output results/27b.csv
python -m hasebench report --format json --output results/comparison.json
```

`report` reads only completed `hasebench-run.json` files in direct children of `work/`; it does not launch agents or revalidate code. It shows a per-model score, per-task summary, and task-by-model outcome table. A model configuration includes the agent, exact OpenCode model selector, backend, and variant. Each score uses the latest saved attempt for that configuration and **task version**. Older attempts remain in CSV/JSON exports, with a `selected` flag in CSV. This preserves distinct failure classifications and prevents different task versions from being silently combined. Scores have their own task counts; compare models on shared task/version rows when their coverage differs. Corrupt run metadata is skipped with a warning.

The 21 C++ tasks are listed in [TASKS.md](TASKS.md). See [progress.md](progress.md) for the current stage.

Prompt example:
```markdown
Read TASK.md and solve the task.

The current directory is the entire project scope for this task. Do not inspect, list, read, or modify parent or ancestor directories, even if they are visible through Git. Do not change files outside the current workspace.

You may inspect this workspace, edit the implementation, configure/build the project, and run the visible tests.

Continue working until you believe the task is complete.

Do not ask me for implementation guidance unless you are genuinely blocked.
```
