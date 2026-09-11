# Hase Bench

The current milestone supports manual C++ benchmark runs only. It does not invoke OpenCode itself.

Run the commands from a Visual Studio 2026 x64 Developer PowerShell (or an
equivalently initialized MSVC environment). `cl.exe` and CMake must be on
`PATH`; the framework deliberately does not select a GCC or Clang fallback.

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

Currently available tasks are CPP-001 (expression evaluator), CPP-003 (generic LRU cache), and CPP-005 (SPSC ring buffer). See [progress.md](progress.md) for the current stage.

Prompt example:
```markdown
Read TASK.md and solve the task.

The current directory is the entire project scope for this task. Do not inspect, list, read, or modify parent or ancestor directories, even if they are visible through Git. Do not change files outside the current workspace.

You may inspect this workspace, edit the implementation, configure/build the project, and run the visible tests.

Continue working until you believe the task is complete.

Do not ask me for implementation guidance unless you are genuinely blocked.
```
