# RUST-004 — Bounded channel worker

**Severity: High.** This task combines synchronized multi-producer admission,
bounded backpressure, a dedicated worker thread, ordered result publication,
draining shutdown, and failure recovery for move-only jobs.

Implement the worker declared in `src/lib.rs`. Use Rust 2024 and only the
standard library. Keep the crate name `rust_004` and preserve the public types
and methods below. Their fields and the worker's internal representation may
remain private except where fields are shown as public.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accepted {
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed<R> {
    pub sequence: u64,
    pub result: R,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Undelivered<J> {
    pub sequence: u64,
    pub job: J,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError<J> {
    Full(J),
    Closed(J),
    Failed(J),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerConfigError {
    ZeroCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStatus<E> {
    Running,
    Closed,
    Failed(E),
}

pub struct BoundedWorker<J, R, E> { /* private fields */ }

impl<J, R, E> BoundedWorker<J, R, E> {
    pub fn new<F>(capacity: usize, handler: F) -> Result<Self, WorkerConfigError>
    where
        J: Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
        F: FnMut(&J) -> Result<R, E> + Send + 'static;

    pub fn try_submit(&self, job: J) -> Result<Accepted, SubmitError<J>>;
    pub fn close(&self);
    pub fn status(&self) -> WorkerStatus<E> where E: Clone;
    pub fn completed(&self) -> Vec<Completed<R>> where R: Clone;
    pub fn take_undelivered(&self) -> Vec<Undelivered<J>>;
}
```

## Admission and ordering

- `capacity` must be positive; zero returns `WorkerConfigError::ZeroCapacity`.
- Exactly one dedicated thread calls the handler.
- `try_submit` never waits for capacity. It returns `SubmitError::Full(job)`
  when all queue slots are occupied, preserving ownership of the original job.
- Capacity counts jobs waiting in the channel. The job currently being handled
  does not occupy a queue slot.
- Successful submissions receive gap-free sequence numbers beginning at zero.
  Full, closed, and failed submissions consume no sequence number.
- Multiple threads may call `try_submit` through a shared worker. Accepted jobs
  are handled and published in sequence order.
- Jobs need only be `Send`; they need not implement `Clone`, `Copy`, or
  `Default`. The handler borrows each job so a failed job can be recovered.

## Results, shutdown, and failure

- `completed()` returns a snapshot of successful results in sequence order.
- `close()` rejects new submissions, drains every accepted job unless the
  handler fails, waits for the worker thread to finish, and may be called
  repeatedly. Concurrent overlapping `close()` calls are outside scope.
- Dropping the worker performs the same draining close.
- Before close or failure, `status()` is `Running`. After a successful close it
  is `Closed`.
- When the handler returns `Err(error)`, processing stops. `status()` becomes
  `Failed(error)`, and later submissions return `SubmitError::Failed(job)`.
- On failure, `take_undelivered()` returns the failed job followed by all
  accepted queued jobs in sequence order. Calling it again returns an empty
  vector. Results completed before the failure remain available.
- A failing operation never publishes a completed result. Handler panics are
  outside this task's contract.

The implementation must not busy-wait or use timing sleeps for correctness.
Build and run the visible tests with:

```text
cargo test --locked
```
