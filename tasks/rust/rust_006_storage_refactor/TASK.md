# RUST-006 — Trait-driven storage refactor

**Severity: High.** This multi-file task combines an object-safe architecture
boundary, transactional state, owned data, deterministic middleware order,
typed failures, and rollback recovery.

Refactor the concrete service in `src/` while preserving its visible legacy
behavior. Use Rust 2024 and only the standard library. Keep the crate name
`rust_006`. Preserve the existing public operation and error types, and add the
public traits and methods specified below. Re-export all listed public items
from the crate root.

## Public storage boundary

```rust
pub trait Transaction {
    fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&mut self, key: &str, value: &[u8])
        -> Result<Option<Vec<u8>>, StorageError>;
    fn delete(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn commit(&mut self) -> Result<(), StorageError>;
    fn rollback(&mut self) -> Result<(), StorageError>;
}

pub trait Storage {
    fn begin(&mut self) -> Result<Box<dyn Transaction + '_>, StorageError>;
}
```

Both traits must be object-safe. The service must use only these trait methods;
it must not downcast, special-case `MemoryStore`, or require hidden backends to
expose task-specific hooks.

`MemoryStore` remains public, cloneable, and default-constructible. It must
implement `Storage` transactionally. Preserve:

```rust
pub fn MemoryStore::from_entries(
    entries: impl IntoIterator<Item = (String, Vec<u8>)>
) -> MemoryStore;

pub fn MemoryStore::snapshot(&self) -> BTreeMap<String, Vec<u8>>;
```

Clones are handles to the same committed store, so a clone retained by the
caller observes later commits. Uncommitted changes are private to a transaction.
After successful commit or rollback, every transaction method returns
`StorageError::TransactionClosed` and cannot change committed state.

## Service and middleware boundary

Preserve `Operation`:

```rust
pub enum Operation {
    Get { key: String },
    Put { key: String, value: Vec<u8> },
    Delete { key: String },
}
```

Add the object-safe middleware trait:

```rust
pub trait Middleware {
    fn handle(
        &mut self,
        phase: MiddlewarePhase,
        operation_index: usize,
        operation: &Operation,
    ) -> Result<(), MiddlewareError>;
}
```

`MiddlewarePhase` has `Before` and `After` variants. `DocumentService` remains
a non-generic public struct so existing code can name the type without a type
parameter. Its constructor, rather than the struct itself, is generic over the
storage implementation. It must provide:

```rust
pub fn DocumentService::new<S: Storage + 'static>(storage: S) -> Self;
pub fn DocumentService::add_middleware<M: Middleware + 'static>(
    &mut self,
    middleware: M,
);
pub fn DocumentService::execute(
    &mut self,
    operations: Vec<Operation>,
) -> Result<Vec<Option<Vec<u8>>>, ServiceError>;
```

The returned vector corresponds one-for-one with the operations. `Get` returns
the current value, `Put` returns the previous value, and `Delete` returns the
removed value. All returned bytes are caller-owned and independent of both the
request and backend storage.

## Validation, order, and transaction semantics

- A valid key is 1–64 bytes and contains only ASCII letters, digits, `_`, `-`,
  or `.`. Validate every key in operation order before beginning a transaction.
  The first invalid key returns `ServiceError::InvalidKey` with its index and
  owned key. No storage or middleware method is called.
- An empty batch returns an empty vector without beginning a transaction.
- Begin one transaction for the whole non-empty batch.
- For each operation, call every middleware `Before` hook in registration
  order, then the corresponding transaction method, then every `After` hook in
  reverse registration order. Finish one operation before starting the next.
- On success, commit once after all operations and return the owned results.
- A middleware or storage-operation failure stops immediately. No later hooks
  or operations run. Call rollback exactly once.
- A commit failure also triggers exactly one rollback attempt.
- If rollback succeeds, return the original typed error. If rollback fails,
  return `ServiceError::Rollback`, retaining the complete original
  `ServiceError` and the rollback `StorageError`.

## Required public errors

Preserve the fields and derives shown in the starter. The required shapes are:

```rust
pub enum StorageError {
    Backend(String),
    Conflict(String),
    TransactionClosed,
}

pub enum MiddlewareError {
    Rejected(String),
}

pub enum OperationFailure {
    Middleware { phase: MiddlewarePhase, source: MiddlewareError },
    Storage(StorageError),
}

pub enum ServiceError {
    InvalidKey { operation: usize, key: String },
    Begin(StorageError),
    Operation { operation: usize, source: OperationFailure },
    Commit(StorageError),
    Rollback { original: Box<ServiceError>, rollback: StorageError },
}
```

The implementation must not use unsafe code or global mutable state. Build and
run the visible tests with:

```text
cargo test --locked
```
