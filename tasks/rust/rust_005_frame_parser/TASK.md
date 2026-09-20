# RUST-005 — Incremental binary frame parser

**Severity: High.** This task combines a bounded incremental state machine,
binary ownership, explicit validation precedence, completed-frame recovery,
and an irreversible error state.

Implement the parser declared in `src/lib.rs`. Use Rust 2024 and only the
standard library. Keep the crate name `rust_005` and preserve the public API.

## Wire format

Each frame is exactly:

```text
offset  size  field
0       2     ASCII magic "HB"
2       1     version, exactly 1
3       1     kind (any u8 value)
4       4     payload length, unsigned big-endian u32
8       N     payload bytes
8 + N   4     checksum, unsigned big-endian u32
```

The checksum is 32-bit FNV-1a. Start with offset basis `2166136261`, then hash
the six header bytes beginning with `version` (version, kind, and all four
encoded length bytes), followed by every payload byte. Arithmetic wraps modulo
2^32. The final checksum field is not included in the hash.

## Public API

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidMagic { index: u8, found: u8 },
    UnsupportedVersion(u8),
    FrameTooLarge { length: u32, max_payload: usize },
    ChecksumMismatch { expected: u32, actual: u32 },
    Truncated { buffered: usize, needed: usize },
    AlreadyFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserStatus {
    Active,
    Finished,
    Failed,
}

pub struct FrameParser { /* private fields */ }

impl FrameParser {
    pub fn new(max_payload: usize) -> Self;
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), ParseError>;
    pub fn finish(&mut self) -> Result<(), ParseError>;
    pub fn take_frames(&mut self) -> Vec<Frame>;
    pub fn status(&self) -> ParserStatus;
    pub fn error(&self) -> Option<&ParseError>;
}
```

`max_payload == 0` is valid and permits only empty payloads.

## Incremental behavior and precedence

- A frame may be split at any byte boundary across any number of `push` calls.
  A single chunk may contain multiple frames and a prefix of another frame.
- Empty chunks are valid while the parser is active.
- Validate fields in wire order: magic byte 0, magic byte 1, version, payload
  length, then checksum. Report a mismatch as soon as the bytes required for
  that check are available. For `InvalidMagic`, `index` is 0 or 1 and `found`
  is the mismatching byte.
- Reject `FrameTooLarge` as soon as the complete eight-byte header is present;
  do not wait for the declared payload or checksum.
- `ChecksumMismatch.expected` is the calculated FNV-1a value and `actual` is
  the value encoded in the frame.
- `take_frames()` returns all completed, not-yet-taken frames in wire order and
  removes them from the parser. It remains usable after failure or finish.
- If a later frame in a chunk is malformed, earlier frames completed from that
  chunk remain available through `take_frames()`. The malformed frame is never
  published.

## Finish and permanent state

- `finish()` declares end of input. With no partial frame buffered, it succeeds,
  changes the status to `Finished`, and is idempotent.
- With an incomplete frame, `finish()` returns `Truncated`. `buffered` is the
  number of bytes retained for that frame. `needed` is 8 while its header is
  incomplete; after a valid complete header, it is the full encoded frame size
  `8 + payload_length + 4`.
- Any malformed input or truncation changes the status permanently to `Failed`.
  `error()` returns the original error. Every later `push()` or `finish()`
  returns that same error without consuming input or discarding completed
  frames.
- After a successful finish, another `finish()` succeeds. Any later `push()`,
  including an empty chunk, returns `AlreadyFinished`; the status stays
  `Finished` and `error()` stays `None`.

The parser must not panic on arbitrary input, use unsafe code, busy-wait, or
use timing sleeps. Build and run the visible tests with:

```text
cargo test --locked
```
