# RUST-007 — Concurrent single-flight cache

**Severity: Very high.** This task combines generic ownership, sharded
synchronization, deterministic time, per-key coordination, panic recovery,
bounded LRU-style eviction, and shutdown races.

Complete the cache in `src/lib.rs`. Use Rust 2024 and only the standard
library. Keep the crate name `rust_007` and preserve the public API below. The
implementation must not use unsafe code or global mutable state.

## Public API

```rust
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    ZeroCapacity,
    ZeroShards,
    MoreShardsThanCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError<E> {
    Loader(E),
    LoaderPanicked,
    ReentrantLoad,
    Closed,
}

pub struct Cache<K, V, E> { /* private fields */ }

impl<K, V, E> Cache<K, V, E>
where
    K: Eq + Hash + Clone,
    E: Clone,
{
    pub fn new<C: Clock>(
        capacity: usize,
        shard_count: usize,
        ttl_ticks: u64,
        clock: C,
    ) -> Result<Self, ConfigError>;

    pub fn get_or_load<F>(&self, key: K, loader: F) -> Result<Arc<V>, CacheError<E>>
    where
        F: FnOnce() -> Result<V, E>;

    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn is_closed(&self) -> bool;
    pub fn close(&self);
}
```

`Cache` must be usable through `Arc<Cache<...>>` by ordinary `Send + Sync`
keys, values, errors, and clocks. Callers supply the clock; the cache must not
read wall-clock time.

## State and concurrency model

Each key is absent, loading, or ready. At most one loader may execute for a key.
Concurrent callers for that key wait for the same flight and receive clones of
the same result (`Arc<V>` on success or `CacheError<E>` on failure). The loader
runs on its caller's thread and never while any cache mutex is held.

Unrelated keys must make progress independently, including keys mapped to the
same shard. A blocked loader must not prevent a ready hit or another key's
loader from completing. Waiting must be blocking synchronization, not polling
or spinning.

If a loader returns `Err(e)`, every caller in that flight receives
`CacheError::Loader(e.clone())`. If it panics, catch the panic and return
`LoaderPanicked` to the leader and all waiters. Errors and panics are not
cached; a later call can retry. If a loader synchronously calls
`get_or_load` on the same cache and key from the same thread, the nested call
returns `ReentrantLoad` instead of deadlocking. Loading another key is allowed.

## TTL and capacity

A successful value expires at `publication_tick.saturating_add(ttl_ticks)`.
It is fresh only while `clock.now() < expires_at`; equality is expired. A zero
TTL result is delivered to its current flight but is not retained. `len` and
`is_empty` purge expired entries before counting.

Capacity counts ready, unexpired entries only; loading entries consume no
capacity. The exact per-shard quota for shard index `i` is
`capacity / shard_count`, plus one when `i < capacity % shard_count`. These
quotas sum to `capacity`, and `new` rejects zero capacity, zero shards, or more
shards than capacity. On a fresh hit, update that entry's recency. After
publishing a successful load, evict least-recently-used ready entries from that
shard until its quota is satisfied. Recency ties may be resolved arbitrarily.

Use a deterministic `DefaultHasher` hash of the key modulo `shard_count` to
select the shard.

## Shutdown

`close` is idempotent. It permanently rejects new calls with `Closed`, removes
all ready values, and wakes every current flight with `Closed`. A loader that
returns after its flight was closed must not publish or repopulate the cache
and also returns `Closed`. If successful publication wins its synchronization
race with `close`, that current flight may return the value, but `close` still
removes it before returning. Dropping the cache performs the same cleanup.

Build and run the visible tests with:

```text
cargo test --locked
```
