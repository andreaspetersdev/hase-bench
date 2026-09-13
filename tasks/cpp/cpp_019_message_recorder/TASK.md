# CPP-019 — Concurrent message recorder

Repair and complete the C++20 recorder in `include/message_recorder.hpp` and
`src/message_recorder.cpp`. Preserve the public API and CMake target names.
Do not add external dependencies.

`MessageRecorder(capacity, sink)` owns one writer thread. `capacity` must be
positive. The optional sink callback models persistence; the default sink does
nothing. The writer calls it once per attempted message, in sequence order,
until the first failure, with no recorder mutex held. The callback must not
call the recorder. The API is safe for concurrent submitters; only one thread
may call `close`, and `close` must not run concurrently with destruction.

`submit(timestamp, bytes)` returns immediately. The caller-supplied timestamp is
stored unchanged; timestamps need not be ordered. Submission accepts only while
the recorder is open, the sink has not failed, and fewer than `capacity`
messages are waiting in the queue. The message currently being written does
not occupy a queue slot. An
accepted message receives the next sequence number, starting at zero; rejected
submissions consume no number. Payloads are arbitrary bytes, including NUL.
The mutex acquisition order of concurrent `submit` calls determines acceptance
and sequence order.

The writer removes queued messages in FIFO order. It invokes the sink before
recording successful persistence. `snapshot()` may be called at any time and
returns copies of successfully persisted messages in sequence order; after
`close` returns, that result is stable.
If the sink throws, the writer catches the exception, marks the recorder failed,
stops writing, and rejects later submissions. `undelivered()` then returns the
failed message followed by every accepted message still queued, in sequence
order. No accepted message may disappear or appear in both results.

`close()` first rejects future submissions, then waits for the writer to finish
all accepted work or report a sink failure, and joins it. It is idempotent.
`closed()` becomes true as soon as `close()` starts rejecting submissions.
Destruction performs the same close. After `close`, `failed()`, `snapshot()`,
and `undelivered()` remain usable. A successful close has an empty undelivered
list. Never hold the recorder mutex while calling the sink or joining the
writer.

Build and run the visible CMake tests before finishing.
