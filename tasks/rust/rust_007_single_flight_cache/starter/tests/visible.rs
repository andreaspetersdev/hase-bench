use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use rust_007::{Cache, CacheError, Clock, ConfigError};

#[derive(Clone, Default)]
struct ManualClock(Arc<AtomicU64>);

impl ManualClock {
    fn set(&self, tick: u64) {
        self.0.store(tick, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[test]
fn caches_successes_until_the_exact_expiry_tick() {
    let clock = ManualClock::default();
    let cache = Cache::<String, String, String>::new(4, 2, 10, clock.clone()).unwrap();
    let loads = AtomicUsize::new(0);

    let first = cache
        .get_or_load("key".to_owned(), || {
            loads.fetch_add(1, Ordering::SeqCst);
            Ok("value".to_owned())
        })
        .unwrap();
    clock.set(9);
    let hit = cache
        .get_or_load("key".to_owned(), || panic!("fresh hit must not load"))
        .unwrap();
    assert!(Arc::ptr_eq(&first, &hit));
    assert_eq!(cache.len(), 1);

    clock.set(10);
    let second = cache
        .get_or_load("key".to_owned(), || {
            loads.fetch_add(1, Ordering::SeqCst);
            Ok("new".to_owned())
        })
        .unwrap();
    assert_eq!(&*second, "new");
    assert_eq!(loads.load(Ordering::SeqCst), 2);
}

#[test]
fn loader_errors_are_typed_and_are_not_cached() {
    let cache = Cache::<u8, usize, &'static str>::new(1, 1, 5, ManualClock::default()).unwrap();
    assert_eq!(
        cache.get_or_load(7, || Err("offline")),
        Err(CacheError::Loader("offline"))
    );
    assert!(cache.is_empty());
    assert_eq!(*cache.get_or_load(7, || Ok(42)).unwrap(), 42);
}

#[test]
fn validates_configuration_and_close_is_permanent() {
    assert!(matches!(
        Cache::<u8, u8, u8>::new(0, 1, 1, ManualClock::default()),
        Err(ConfigError::ZeroCapacity)
    ));
    assert!(matches!(
        Cache::<u8, u8, u8>::new(1, 0, 1, ManualClock::default()),
        Err(ConfigError::ZeroShards)
    ));
    assert!(matches!(
        Cache::<u8, u8, u8>::new(1, 2, 1, ManualClock::default()),
        Err(ConfigError::MoreShardsThanCapacity)
    ));

    let cache = Cache::<u8, u8, u8>::new(1, 1, 1, ManualClock::default()).unwrap();
    cache.close();
    cache.close();
    assert!(cache.is_closed());
    assert_eq!(cache.get_or_load(1, || Ok(1)), Err(CacheError::Closed));
}
