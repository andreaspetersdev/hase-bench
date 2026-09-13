# CPP-020 — Integrated mini service

Repair and extend this C++20 service. Preserve the public class and function
signatures, CMake target names, and the legacy record format. You may change
private fields and refactor across files. Do not add external dependencies.

The service accepts chunks of newline-terminated sensor records, sends accepted
events to a dedicated sink worker, and reports statistics after successful sink
delivery. The existing project compiles but mishandles some input, ownership,
queue, failure, and shutdown cases.

Implement source-aware alerting across the configuration, record parser, queue,
and statistics. A legacy record is `timestamp,value\n` and has source
`default`. A tagged record is `timestamp,source,value\n`. A final record
requires LF; CRLF is also accepted. Chunks may split anywhere, including
inside fields and before LF. Empty records, stray CR, and malformed fields are
invalid. A source begins with a lower-case ASCII letter and continues with
lower-case letters, digits, or underscore. Timestamp is unsigned decimal
`uint64_t`; value is unsigned decimal in `[0,1000000]`. Signs and spaces
are invalid. The parser's line limit counts bytes before LF, including CR.

Configuration text uses LF-separated `key=value` lines, with an optional
final LF and no empty lines. It must contain exactly one each of `capacity`
(`1..32`), `max_line_bytes` (`16..256`), and
`default_threshold` (`0..1000000`). It may contain one
`threshold.source=value` for each valid source name; duplicates, unknown
keys, invalid numbers, and out-of-range values invalidate the configuration.
Threshold lookup uses an override when present and otherwise the default.
The returned `Config` must own everything needed for later lookup, even if
the configuration input is changed or destroyed.

`Service::ingest(chunk)` is called by one producer thread. It returns true
only when the entire chunk can be accepted. Parsing and queue admission are
transactional: if any complete line is malformed or the entire batch cannot
fit in the pending queue, no event from that call is accepted and the parser's
partial-line state is unchanged. The caller can retry a rejected chunk.
Accepted events are queued in order. Capacity counts waiting events only;
the event currently in the sink is not a queue slot. An empty or partial-only
chunk may be accepted while the service is open. After close or sink failure,
ingest returns false.

The worker invokes the optional sink once per attempted event, in order, with
no service or queue lock held. The default sink succeeds without side effects.
Only after the sink succeeds does that event enter statistics. An alert occurs
when `value >=` that event's configured threshold. Snapshots contain totals
and per-source rows sorted by source. If the sink throws, the worker stops;
`failed()` becomes true and `undelivered()` returns the failed event then
all accepted queued events in order. None of those events enter statistics.
The sink must not call back into the service.

`close()` rejects future ingestion, drains accepted work unless the sink has
failed, and joins the worker. It is idempotent; destruction has the same
effect. Only one thread may call close, and close must not overlap ingest or
destruction. Snapshot, failed, and undelivered may be queried after close.
An incomplete trailing line is discarded on close. Query calls may run while
the worker is active.

Build and run the visible tests before finishing.
