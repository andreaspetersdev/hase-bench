# RUST-008 — Cancellation-safe async service

**Severity: Very high.** This task combines bounded asynchronous admission,
per-stream ordering, cross-stream multiplexing, cancellation, deterministic
deadlines, graceful shutdown, and permanent transport-failure handling.

Complete the service in `src/lib.rs`. Use Rust 2024 and Tokio **1.47.1**, as
pinned by `Cargo.toml` and `Cargo.lock`. Keep the crate name `rust_008` and
preserve the public API below. Do not use unsafe code, wall-clock time, detached
blocking threads, or additional dependencies.

## Public API

```rust
pub type BoxFuture<'a, T> =
    Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> u64;
    fn sleep_until(&self, deadline: u64) -> BoxFuture<'static, ()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub stream_id: u64,
    pub payload: Vec<u8>,
    pub deadline: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportRequest {
    pub sequence: u64,
    pub stream_id: u64,
    pub payload: Vec<u8>,
}

pub trait Transport: Send + Sync + 'static {
    type Error: Clone + Send + Sync + 'static;

    fn send(&self, request: TransportRequest)
        -> BoxFuture<'_, Result<Vec<u8>, Self::Error>>;
    fn close(&self) -> BoxFuture<'_, Result<(), Self::Error>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError { ZeroCapacity, CapacityTooLarge }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitError<E> {
    Full(Request),
    Closed(Request),
    Failed { request: Request, error: E },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError<E> { Cancelled, DeadlineExceeded, Transport(E) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub sequence: u64,
    pub stream_id: u64,
    pub payload: Vec<u8>,
}

pub struct RequestHandle<E> { /* private fields */ }

impl<E> Future for RequestHandle<E> {
    type Output = Result<Response, CallError<E>>;
}

pub struct Service<E> { /* private fields */ }

impl<E: Clone + Send + Sync + 'static> Service<E> {
    pub fn new<T: Transport<Error = E>, C: Clock>(
        capacity: usize,
        transport: T,
        clock: C,
    ) -> Result<Self, ConfigError>;

    pub fn try_start(&self, request: Request)
        -> Result<RequestHandle<E>, SubmitError<E>>;

    pub fn in_flight(&self) -> usize;
    pub fn is_closed(&self) -> bool;
    pub fn failure(&self) -> Option<E>;
    pub async fn close(&self) -> Result<(), E>;
}
```

`RequestHandle<E>` must be `Send` when `E` is `Send`. `Service<E>` must be
`Send + Sync` and may be shared through `Arc`. `new` and `try_start` are called
inside a Tokio runtime. Sequence numbers begin at zero and are assigned only
to accepted requests; admission failures do not consume one.

## State and concurrency model

`capacity` is the exact number of accepted requests that may be queued or
executing. `try_start` is non-blocking: it returns `Full` with the original
request when no permit is available. Zero capacity and capacities larger than
`u32::MAX` are rejected.

Accepted requests on one `stream_id` reach `Transport::send` in sequence-number
order, with at most one active send for that stream. Different streams are
independent: a blocked stream must not prevent another stream from reaching
the transport. The owned `TransportRequest` contains the accepted sequence and
payload; the deadline is service metadata and is not sent.

Dropping a pending `RequestHandle` cancels its request. Cancellation works both
while queued and while `send` is pending: the send future is dropped, the
capacity permit is released, and the next request on that stream may proceed.
The transport need not receive a request cancelled before its turn. Polling a
handle to completion yields the response/error and must not subsequently
cancel anything. Internal task loss may map to `CallError::Cancelled`.

## Deadlines

The injected clock is the only time source. A request may start sending only
while `clock.now() < deadline`; equality is expired. Its single
`sleep_until(deadline)` future covers both queueing and transport execution.
At the deadline the result is `DeadlineExceeded`, a pending send is dropped,
the permit is released, and the stream advances. A completed transport result
that wins the runtime's synchronization race may be returned.

## Failure and shutdown

The first `Transport::send` error is permanent. That call returns
`CallError::Transport(error)`. Accepted requests that have not started sending
also return that same error, later submissions return `SubmitError::Failed`,
and `failure()` exposes it. Already active sends on other streams may finish.

`close` is graceful and idempotent. It immediately prevents new admission,
waits for every accepted request to finish, then calls `Transport::close`
exactly once. It returns the first send error if one exists; otherwise a close
error becomes the permanent failure and is returned. Concurrent callers wait
for and clone the same result. Closing an idle service is valid. Dropping the
last `Service` value is not a substitute for awaiting `close`; accepted tasks
remain self-contained and release their resources when they finish.

Build and run the visible tests with:

```text
cargo test --locked
```
