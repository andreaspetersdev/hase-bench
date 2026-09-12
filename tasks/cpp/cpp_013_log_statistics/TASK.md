# CPP-013 — Streaming log statistics

Complete the C++20 implementation declared in `include/log_statistics.hpp`.
Do not change the public API, CMake target names, or add external dependencies.

`LogStatistics` receives physical log lines one at a time. Its constructor
defines the inclusive timestamp window to aggregate. It must retain only
bounded state: fixed counters, a compensated aggregate for the mean, and at
most `percentile_capacity` latency values. It must not retain every accepted
line or every latency ever seen.

## Input format

After removing one optional terminal `\r`, a valid line has exactly this form:

```text
<timestamp> <level> <latency_ms> <message>
```

- There is no leading whitespace. The three separators are one or more ASCII
  spaces. `message` is non-empty and is otherwise opaque; it may contain
  spaces (including trailing spaces).
- `timestamp` is a non-negative base-10 integer representable by `int64_t`.
- `level` is exactly `INFO`, `WARN`, or `ERROR`.
- `latency_ms` is a finite, non-negative decimal written as digits optionally
  followed by `.` and zero or more digits. It has no sign, exponent, NaN, or
  infinity spelling. For example, `12`, `12.`, and `12.50` are valid; `.5` is
  not. Convert the complete decimal to the nearest representable `double`;
  do not silently discard later fractional digits.
- Any embedded `\r` or `\n`, a missing field/message, an overflow, or any
  other spelling is malformed.

Malformed lines increment only `malformed`; they do not increment
`outside_window` or any accepted counters. A syntactically valid line whose
timestamp is outside the constructor's inclusive `[window_start, window_end]`
increments only `outside_window` (and `received`). A valid in-window line is
accepted and contributes to all applicable aggregates.

## Statistics

- `received` counts every `ingest` call. `accepted`, `malformed`, and
  `outside_window` have the meanings above. Severity counters count accepted
  lines only.
- `mean_latency_ms` is the mean of every accepted latency, not merely the
  percentile sample. Use compensated summation (or an equivalently stable
  online method); ordinary naive accumulation is insufficient.
- Keep a FIFO sample of the most recent `percentile_capacity` *accepted lines
  in ingestion order*. `p50_latency_ms` and `p95_latency_ms` are calculated
  from a sorted copy of that current sample using the nearest-rank rule:
  index `ceil(p * n) - 1`, with `p = 0.50` or `0.95`. Thus for one sample both
  percentiles equal it. Values outside the timestamp window do not advance
  this FIFO.
- If no line has been accepted, all three floating-point statistic fields are
  zero. Constructor arguments with `window_start > window_end` or
  `percentile_capacity == 0` throw `std::invalid_argument`.

The public result contains no references into the input lines. Build and run
the visible tests before finishing:

```powershell
cmake -S . -B build
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```
