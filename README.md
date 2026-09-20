# Hase Bench

Hase Bench is a reproducible benchmark for evaluating local language models as
software-development agents. It gives an agent a clean copy of a realistic
programming task, then independently builds and tests the result—including
with hidden tests—so model configurations can be compared by correctness,
failure mode, runtime, and generation telemetry.

The current milestone supports manual and autonomous C++ and Rust benchmark
runs through OpenCode, with models served separately from the Windows benchmark
workstation.

## Contents

- [Setup](#setup)
- [Manual Test Run](#manual-test-run)
- [Automatic Test Run](#automatic-test-run)
- [Report](#report)

## Setup

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

Rust tasks additionally require a Rust toolchain. The validator uses `cargo`
from `PATH` when available and otherwise checks the standard per-user
`CARGO_HOME`/`.cargo/bin` installation.

## Manual Test Run

Manual runs let you interact with OpenCode yourself before Hase Bench performs
the authoritative validation. The canonical task remains unchanged; all agent
work happens in a fresh workspace under `work/`.

### Select and prepare a task

From the repository root:

```powershell
python -m hasebench list
python -m hasebench info cpp_001
python -m hasebench prepare cpp_001
python -m hasebench prepare cpp_001 --label A3B
```

`list` shows the catalogue, `info` prints one task's metadata, and `prepare`
prints the new workspace path. The optional label identifies the model or run
without changing the benchmark configuration.

### Run the coding agent

Enter the printed workspace, launch OpenCode, and ask it to read `TASK.md`.
A complete prompt example appears at the end of this README.

### Validate the result

When the agent is finished, validate the workspace independently:

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

For Rust tasks, both `starter/` and
`validator/` must contain committed `Cargo.toml` and `Cargo.lock` files, and the
starter package name must equal the task ID. The validator crate declares that
package as a dependency. During authoritative validation, the framework uses a
Cargo source patch to redirect the dependency to the isolated run workspace,
so hidden test sources stay in the canonical `validator/` directory. Cargo
build artifacts are retained under the workspace's `build/` directory.

## Automatic Test Run

Automatic runs create a fresh workspace, invoke OpenCode non-interactively,
and then run the same authoritative validation used by manual runs.

### Run one task

Run one task in a fresh autonomous workspace:

```powershell
python -m hasebench run cpp_001 --agent opencode --model hase/qwen27b-q4 --label 27BQ4
python -m hasebench run cpp_001 --agent opencode --model llama-hase/qwen3.8-27b --backend llama.cpp --variant xhigh --output-token-max 65536 --label 27B-xhigh
python -m hasebench run cpp_001 --agent opencode --model vllm-hase//home/ape/models/hf/Qwen3.8-27B-FP8 --backend vllm --variant medium --label 27BFP8
```

### Run multiple tasks

Run every available task, or limit an all-task run to a task ID:

```powershell
python -m hasebench run --all --agent opencode --model hase/qwen27b-q4 --label 27BQ4
python -m hasebench run --all --task cpp_003 --agent opencode --model hase/qwen27b-q4
```

### Command options

| Parameter | Required | Purpose | Example |
| --- | --- | --- | --- |
| `TASK` or `--all` | Yes | Run one task, or every available task. | `cpp_001`, `--all` |
| `--task` | With `--all`, optional | Restrict a batch to one task ID. | `--task cpp_003` |
| `--agent` | Yes | Coding agent to launch. | `--agent opencode` |
| `--model` | Yes | OpenCode provider/model selector, passed unchanged to OpenCode. | `--model llama-hase/qwen3.8-27b` |
| `--variant` | No | Provider-specific reasoning effort, passed as OpenCode's `--variant`. | `--variant medium`, `--variant xhigh` |
| `--output-token-max` | No | Positive per-response token ceiling passed only to the OpenCode child through `OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX`. | `--output-token-max 65536` |
| `--model-name` | No | Descriptive model name retained in metadata; defaults to `--model`. | `--model-name Qwen-3.8-27B` |
| `--backend` | No | Backend label retained in metadata and reports. | `--backend llama.cpp` |
| `--label` | No | Safe suffix for each fresh workspace name. | `--label 27B-xhigh` |
| `--timeout` | No | Agent time limit in seconds; default is 1,800 (30 minutes). | `--timeout 1800` |
| `--verbose` | No | Print OpenCode, compiler, and test diagnostics. | `--verbose` |

### Configuration and generated artifacts

Configure the provider, endpoint, credentials, and any model alias in OpenCode; the benchmark does not hard-code hase connection settings. The selected variant, model/backend labels, and timing are retained in run metadata and reports.
`--model` must be OpenCode's `provider/model` selector. A server-side path such as `/home/ape/models/hf/Qwen3.8-27B-FP8` is only the model ID; for the configured `vllm-hase` provider, OpenCode lists the full selector as `vllm-hase//home/ape/models/hf/Qwen3.8-27B-FP8`. Check `opencode models vllm-hase` after changing the provider configuration.

The current OpenCode default applies a 32,000-token per-response output ceiling,
which may be lower than a model's configured `limit.output`. Use
`--output-token-max` to make
that ceiling explicit and reproducible for an autonomous run. Reasoning tokens
and visible output share this allowance, so reasoning-heavy models can reach it
without producing a tool call. The selected value must also fit the inference
server's actual context window after the request prompt is included; raising it
does not enlarge the model or server context. When the option is omitted, the
framework removes any ambient `OPENCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX` override
from the child environment and uses OpenCode's default behavior.

Each autonomous run creates a new `_aut_opencode` workspace, runs OpenCode with non-interactive JSON output and permission auto-approval inside that workspace, then validates it with the same visible and hidden CMake validators as manual runs. `TEMP` and `TMP` are redirected into the workspace so ordinary agent-created temporary files are preserved there too. The workspace is always preserved. It contains `hasebench-agent.log` and `hasebench-run.json`, recording the selected configuration, model/backend labels, output-token maximum, agent exit status/timing, validation commands/results, and final classification. Use `--verbose` to print captured agent and validation diagnostics.

After every autonomous task, the console prints its task ID and title, workspace, model, build, visible-test, hidden-test, and final result. It also prints OpenCode's maximum observed context use, generated-token rate, estimated model execution time, agent elapsed time, and full execution time. Model execution time is calculated from OpenCode step timing with recorded tool time removed; full execution time includes workspace preparation, the agent run, and authoritative validation. If an OpenCode version does not emit JSON telemetry, those fields are explicitly shown as unavailable. Every command also writes a Markdown summary table under `results/`; `run --all` produces one aggregate table for the batch, while a single-task run produces a one-row table. The Markdown table and per-run JSON metadata both retain the task title. These generated reports are ignored by Git.

## Report

Use `report` to compare saved autonomous runs. It never launches an agent or
revalidates a workspace.

### View a report

```powershell
python -m hasebench report
python -m hasebench report --task cpp_021
python -m hasebench report --model llama-hase/qwen3.8-27b --format csv --output results/27b.csv
python -m hasebench report --format json --output results/comparison.json
```

### Report contents

`report` reads only completed `hasebench-run.json` files in direct children of
`work/`. Its on-screen format contains exactly three aligned tables: numbered
model settings, task severity and description, and results by task/model ID.
Result rows include maximum observed context, generated tokens, generation
speed, full execution time, and calculated model execution time; unavailable
fields remain explicit. Short model IDs keep exact provider selectors and
backend settings out of the repeated result rows. The model-settings table also
shows the OpenCode output-token maximum because runs with different response
ceilings are different benchmark configurations.

Each result uses the latest saved attempt for that configuration and **task
version**. Older attempts remain in CSV/JSON exports, with a `selected` flag in
CSV. The detailed exports retain configuration, failure, timing, token, and
speed fields. This preserves distinct failure classifications and prevents
different task versions from being silently combined. Corrupt run metadata is
skipped with a warning.

## Task catalogue and agent prompt

The 21 C++ tasks and initial Rust tasks are listed in [TASKS.md](TASKS.md). See [progress.md](progress.md) for the current stage.

Prompt example:
```markdown
Read TASK.md and solve the task.

The current directory is the entire project scope for this task. Do not inspect, list, read, or modify parent or ancestor directories, even if they are visible through Git. Do not change files outside the current workspace.

You may inspect this workspace, edit the implementation, configure/build the project, and run the visible tests.

Continue working until you believe the task is complete.

Do not ask me for implementation guidance unless you are genuinely blocked.
```
