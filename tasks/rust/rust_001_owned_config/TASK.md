# RUST-001 — Owned configuration snapshot repair

**Severity: Medium.** This task focuses on one ownership/lifetime boundary plus
a compact parsing contract. It does not require concurrency, asynchronous code,
external dependencies, or cross-component recovery.

The crate parses a small line-oriented configuration format. Its current
implementation keeps string slices into the caller's input. Repair the public
types and implementation so a parsed `ConfigSnapshot` owns all of its data.

Use Rust 2024. Keep the crate name `rust_001` and preserve these public types
and operations (removing the current lifetime parameters is part of the task):

```rust
pub enum ConfigError {
    MissingEquals { line: usize },
    EmptyKey { line: usize },
    DuplicateKey { line: usize, key: String },
}

pub struct ConfigEntry { /* owned key and value */ }

impl ConfigEntry {
    pub fn key(&self) -> &str;
    pub fn value(&self) -> &str;
}

pub struct ConfigSnapshot { /* owned entries */ }

impl ConfigSnapshot {
    pub fn parse(input: &str) -> Result<Self, ConfigError>;
    pub fn get(&self, key: &str) -> Option<&str>;
    pub fn set(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String>;
    pub fn entries(&self) -> &[ConfigEntry];
}
```

`ConfigEntry`, `ConfigSnapshot`, and `ConfigError` must remain `Debug`,
`Clone`, `PartialEq`, and `Eq`.

## Parsing rules

- Input is split at `\n`. A `\r` immediately before `\n` is not part of the
  line. A final line need not have a terminator.
- Empty lines and lines containing only ASCII space or tab are ignored.
- Every other line is split at its first `=`. A missing `=` is
  `ConfigError::MissingEquals`.
- ASCII spaces and tabs surrounding the key and value are removed. Other
  characters are data, and further `=` characters belong to the value.
- An empty trimmed key is `ConfigError::EmptyKey`.
- Keys are compared with ASCII case-insensitive semantics. A duplicate is
  `ConfigError::DuplicateKey` containing the one-based physical line number
  and the duplicate line's trimmed key spelling.
- Successful parsing preserves entry order and the original trimmed spelling
  of keys and values.

`get` uses the same ASCII case-insensitive key comparison. `set` updates an
existing entry without changing its key spelling or position and returns the
previous owned value. A new key is appended and returns `None`. Callers provide
non-empty keys to `set`.

The snapshot must remain valid after the input string is changed or destroyed.
`set` must take ownership of owned arguments, rather than borrow them. Cloning
a snapshot must create independent owned state: changing either clone must not
change the other.

Build the crate and run the visible tests with:

```text
cargo test --locked
```
