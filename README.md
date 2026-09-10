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
```

Enter the printed workspace, launch OpenCode yourself, and ask it to read `TASK.md`. When finished, validate independently:

```powershell
python -m hasebench validate <workspace>
```

Use `--verbose` to print captured CMake and CTest diagnostics. Preparation copies only `starter/` and `TASK.md`; hidden validators remain under canonical `tasks/` and are never placed in the workspace.

Currently available tasks are CPP-001 (expression evaluator), CPP-003 (generic LRU cache), and CPP-005 (SPSC ring buffer). See [progress.md](progress.md) for the current stage.
