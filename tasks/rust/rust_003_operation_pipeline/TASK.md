# RUST-003 — Context-rich operation pipeline

**Severity: Low.** This task exercises typed error propagation and sequential
state updates through a compact API. It does not require concurrency,
asynchronous code, external dependencies, or transactional rollback.

Implement the counter-update pipeline declared in `src/lib.rs`. A pipeline
parses textual operations, looks up an existing counter, applies a checked
signed update, and reports the new value after every successful operation.

Use Rust 2024. Keep the crate name `rust_003` and preserve this public API:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterStore { /* private fields */ }

impl CounterStore {
    pub fn new() -> Self;
    pub fn insert(&mut self, key: impl Into<String>, value: i64) -> Option<i64>;
    pub fn get(&self, key: &str) -> Option<i64>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOperationError {
    MissingEquals,
    EmptyKey,
    InvalidDelta { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    MissingKey { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    Overflow { key: String, current: i64, delta: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    Parse(ParseOperationError),
    Lookup(LookupError),
    Update(UpdateError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineError {
    pub operation_index: usize,
    pub error: OperationError,
}

pub fn apply_operations(
    store: &mut CounterStore,
    operations: &[&str],
) -> Result<Vec<i64>, PipelineError>;
```

`OperationError` must implement `From<ParseOperationError>`,
`From<LookupError>`, and `From<UpdateError>`. All five error types must
implement `std::fmt::Display` and `std::error::Error`.
`PipelineError::source()` returns its `OperationError`;
`OperationError::source()` returns the typed error held by its variant. The
three leaf errors have no source.

## Operation rules

- Each operation has the form `key=delta` and is split at its first `=`.
- Remove ASCII spaces and tabs around the key and delta. Other characters are
  data.
- A missing `=` is `ParseOperationError::MissingEquals`.
- An empty trimmed key is `ParseOperationError::EmptyKey`.
- Parse the complete trimmed delta as a base-10 `i64`. Empty, out-of-range, or
  otherwise invalid text is `ParseOperationError::InvalidDelta`, containing
  that trimmed text.
- Keys are case-sensitive. A key not already in the store is
  `LookupError::MissingKey`, containing the trimmed key.
- Addition uses `i64::checked_add`. Overflow is `UpdateError::Overflow`,
  containing the key, old value, and delta; the stored value is unchanged.
- A successful operation updates the store and contributes its new value to
  the returned vector.

Operations run in slice order. On failure, stop immediately and return its
zero-based `operation_index`. Earlier successful updates remain applied; the
failing operation and later operations make no changes.

Within one operation, error precedence is missing separator, empty key,
invalid delta, missing key, then overflow. Parsing therefore completes before
lookup. The first failing operation always wins. Empty input succeeds with an
empty result.

Build the crate and run the visible tests with:

```text
cargo test --locked
```
