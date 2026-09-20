use std::hash::Hash;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

pub struct Cache<K, V, E> {
    closed: AtomicBool,
    _types: PhantomData<fn(K) -> (V, E)>,
}

impl<K, V, E> Cache<K, V, E>
where
    K: Eq + Hash + Clone,
    E: Clone,
{
    pub fn new<C: Clock>(
        capacity: usize,
        shard_count: usize,
        _ttl_ticks: u64,
        _clock: C,
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
        Ok(Self {
            closed: AtomicBool::new(false),
            _types: PhantomData,
        })
    }

    pub fn get_or_load<F>(&self, _key: K, loader: F) -> Result<Arc<V>, CacheError<E>>
    where
        F: FnOnce() -> Result<V, E>,
    {
        if self.is_closed() {
            return Err(CacheError::Closed);
        }
        match catch_unwind(AssertUnwindSafe(loader)) {
            Ok(Ok(value)) => Ok(Arc::new(value)),
            Ok(Err(error)) => Err(CacheError::Loader(error)),
            Err(_) => Err(CacheError::LoaderPanicked),
        }
    }

    pub fn len(&self) -> usize {
        0
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

impl<K, V, E> Drop for Cache<K, V, E> {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
    }
}
