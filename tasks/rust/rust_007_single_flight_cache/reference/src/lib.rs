use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, ThreadId};

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

struct Ready<V> {
    value: Arc<V>,
    expires_at: u64,
    recency: u64,
}

enum Entry<V, E> {
    Loading(Arc<Flight<V, E>>),
    Ready(Ready<V>),
}

struct Flight<V, E> {
    owner: ThreadId,
    result: Mutex<Option<Result<Arc<V>, CacheError<E>>>>,
    wake: Condvar,
}

impl<V, E: Clone> Flight<V, E> {
    fn new() -> Self {
        Self {
            owner: thread::current().id(),
            result: Mutex::new(None),
            wake: Condvar::new(),
        }
    }

    fn complete(&self, result: Result<Arc<V>, CacheError<E>>) -> Result<Arc<V>, CacheError<E>> {
        let mut slot = lock(&self.result);
        if slot.is_none() {
            *slot = Some(result);
            self.wake.notify_all();
        }
        slot.as_ref().expect("flight completed").clone()
    }

    fn wait(&self) -> Result<Arc<V>, CacheError<E>> {
        let mut slot = lock(&self.result);
        while slot.is_none() {
            slot = self
                .wake
                .wait(slot)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        slot.as_ref().expect("flight completed").clone()
    }
}

struct Shard<K, V, E> {
    entries: HashMap<K, Entry<V, E>>,
    capacity: usize,
}

struct Inner<K, V, E> {
    shards: Vec<Mutex<Shard<K, V, E>>>,
    clock: Arc<dyn Clock>,
    ttl_ticks: u64,
    recency: AtomicU64,
    closed: AtomicBool,
}

pub struct Cache<K, V, E> {
    inner: Inner<K, V, E>,
}

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
    ) -> Result<Self, ConfigError> {
        if capacity == 0 {
            return Err(ConfigError::ZeroCapacity);
        }
        if shard_count == 0 {
            return Err(ConfigError::ZeroShards);
        }
        if shard_count > capacity {
            return Err(ConfigError::MoreShardsThanCapacity);
        }
        let base = capacity / shard_count;
        let extra = capacity % shard_count;
        let shards = (0..shard_count)
            .map(|index| {
                Mutex::new(Shard {
                    entries: HashMap::new(),
                    capacity: base + usize::from(index < extra),
                })
            })
            .collect();
        Ok(Self {
            inner: Inner {
                shards,
                clock: Arc::new(clock),
                ttl_ticks,
                recency: AtomicU64::new(0),
                closed: AtomicBool::new(false),
            },
        })
    }

    pub fn get_or_load<F>(&self, key: K, loader: F) -> Result<Arc<V>, CacheError<E>>
    where
        F: FnOnce() -> Result<V, E>,
    {
        if self.is_closed() {
            return Err(CacheError::Closed);
        }
        let shard_index = self.shard_index(&key);
        let flight = {
            let now = self.inner.clock.now();
            let mut shard = lock(&self.inner.shards[shard_index]);
            if self.is_closed() {
                return Err(CacheError::Closed);
            }
            if let Some(entry) = shard.entries.get_mut(&key) {
                match entry {
                    Entry::Ready(ready) if now < ready.expires_at => {
                        ready.recency = self.next_recency();
                        return Ok(Arc::clone(&ready.value));
                    }
                    Entry::Ready(_) => {
                        shard.entries.remove(&key);
                    }
                    Entry::Loading(existing) => {
                        if existing.owner == thread::current().id() {
                            return Err(CacheError::ReentrantLoad);
                        }
                        let existing = Arc::clone(existing);
                        drop(shard);
                        return existing.wait();
                    }
                }
            }
            let flight = Arc::new(Flight::new());
            shard
                .entries
                .insert(key.clone(), Entry::Loading(Arc::clone(&flight)));
            flight
        };

        let loaded = match catch_unwind(AssertUnwindSafe(loader)) {
            Ok(Ok(value)) => Ok(Arc::new(value)),
            Ok(Err(error)) => Err(CacheError::Loader(error)),
            Err(_) => Err(CacheError::LoaderPanicked),
        };
        self.publish(shard_index, key, flight, loaded)
    }

    pub fn len(&self) -> usize {
        let now = self.inner.clock.now();
        self.inner
            .shards
            .iter()
            .map(|mutex| {
                let mut shard = lock(mutex);
                shard.entries.retain(
                    |_, entry| !matches!(entry, Entry::Ready(ready) if now >= ready.expires_at),
                );
                shard
                    .entries
                    .values()
                    .filter(|entry| matches!(entry, Entry::Ready(_)))
                    .count()
            })
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);
        for mutex in &self.inner.shards {
            let mut shard = lock(mutex);
            for (_, entry) in shard.entries.drain() {
                if let Entry::Loading(flight) = entry {
                    let _ = flight.complete(Err(CacheError::Closed));
                }
            }
        }
    }

    fn publish(
        &self,
        shard_index: usize,
        key: K,
        flight: Arc<Flight<V, E>>,
        loaded: Result<Arc<V>, CacheError<E>>,
    ) -> Result<Arc<V>, CacheError<E>> {
        let mut shard = lock(&self.inner.shards[shard_index]);
        let owns_entry = matches!(
            shard.entries.get(&key),
            Some(Entry::Loading(current)) if Arc::ptr_eq(current, &flight)
        );
        if !owns_entry || self.is_closed() {
            if owns_entry {
                shard.entries.remove(&key);
            }
            return flight.complete(Err(CacheError::Closed));
        }

        match loaded {
            Ok(value) => {
                let result = flight.complete(Ok(Arc::clone(&value)));
                if self.inner.ttl_ticks == 0 {
                    shard.entries.remove(&key);
                } else {
                    let now = self.inner.clock.now();
                    shard.entries.insert(
                        key,
                        Entry::Ready(Ready {
                            value,
                            expires_at: now.saturating_add(self.inner.ttl_ticks),
                            recency: self.next_recency(),
                        }),
                    );
                    evict(&mut shard, now);
                }
                result
            }
            Err(error) => {
                let result = flight.complete(Err(error));
                shard.entries.remove(&key);
                result
            }
        }
    }

    fn shard_index(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize % self.inner.shards.len()
    }

    fn next_recency(&self) -> u64 {
        self.inner.recency.fetch_add(1, Ordering::Relaxed)
    }
}

impl<K, V, E> Drop for Cache<K, V, E> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Ordering::Release);
        for mutex in &mut self.inner.shards {
            let shard = mutex
                .get_mut()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for (_, entry) in shard.entries.drain() {
                if let Entry::Loading(flight) = entry {
                    let mut result = flight
                        .result
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if result.is_none() {
                        *result = Some(Err(CacheError::Closed));
                        flight.wake.notify_all();
                    }
                }
            }
        }
    }
}

fn evict<K, V, E>(shard: &mut Shard<K, V, E>, now: u64)
where
    K: Eq + Hash + Clone,
{
    shard
        .entries
        .retain(|_, entry| !matches!(entry, Entry::Ready(ready) if now >= ready.expires_at));
    while shard
        .entries
        .values()
        .filter(|entry| matches!(entry, Entry::Ready(_)))
        .count()
        > shard.capacity
    {
        let oldest = shard
            .entries
            .iter()
            .filter_map(|(key, entry)| match entry {
                Entry::Ready(ready) => Some((key.clone(), ready.recency)),
                Entry::Loading(_) => None,
            })
            .min_by_key(|(_, recency)| *recency)
            .map(|(key, _)| key)
            .expect("ready entry exceeds capacity");
        shard.entries.remove(&oldest);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
