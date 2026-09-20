use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, mpsc};
use std::thread;
use std::time::Duration;

use rust_007::{Cache, CacheError, Clock, ConfigError};

const DEADLOCK_LIMIT: Duration = Duration::from_secs(3);

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
fn public_types_have_the_required_traits_and_cache_is_shareable() {
    fn values<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}
    fn send_sync<T: Send + Sync>() {}
    values::<ConfigError>();
    values::<CacheError<String>>();
    send_sync::<Cache<String, Vec<u8>, String>>();
}

#[test]
fn one_successful_loader_serves_a_concurrent_cohort_with_one_arc() {
    const CALLERS: usize = 8;
    let cache =
        Arc::new(Cache::<String, usize, String>::new(2, 1, 100, ManualClock::default()).unwrap());
    let start = Arc::new(Barrier::new(CALLERS));
    let loads = Arc::new(AtomicUsize::new(0));
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(std::sync::Mutex::new(release_rx));
    let mut threads = Vec::new();
    for _ in 0..CALLERS {
        let cache = Arc::clone(&cache);
        let start = Arc::clone(&start);
        let loads = Arc::clone(&loads);
        let release_rx = Arc::clone(&release_rx);
        threads.push(thread::spawn(move || {
            start.wait();
            cache
                .get_or_load("shared".to_owned(), || {
                    let attempt = loads.fetch_add(1, Ordering::SeqCst);
                    if attempt == 0 {
                        release_rx.lock().unwrap().recv().unwrap();
                    }
                    Ok(91)
                })
                .unwrap()
        }));
    }
    release_tx.send(()).unwrap();
    let values: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert!(values.iter().all(|value| **value == 91));
    assert!(
        values[1..]
            .iter()
            .all(|value| Arc::ptr_eq(&values[0], value))
    );
}

#[test]
fn loader_error_is_shared_by_the_flight_then_a_later_call_retries() {
    const CALLERS: usize = 6;
    let cache = Arc::new(Cache::<u8, u8, String>::new(1, 1, 10, ManualClock::default()).unwrap());
    let start = Arc::new(Barrier::new(CALLERS));
    let loads = Arc::new(AtomicUsize::new(0));
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Arc::new(std::sync::Mutex::new(release_rx));
    let mut threads = Vec::new();
    for _ in 0..CALLERS {
        let cache = Arc::clone(&cache);
        let start = Arc::clone(&start);
        let loads = Arc::clone(&loads);
        let release_rx = Arc::clone(&release_rx);
        threads.push(thread::spawn(move || {
            start.wait();
            cache.get_or_load(4, || {
                let attempt = loads.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    release_rx.lock().unwrap().recv().unwrap();
                }
                Err("failed".to_owned())
            })
        }));
    }
    release_tx.send(()).unwrap();
    for thread in threads {
        assert_eq!(
            thread.join().unwrap(),
            Err(CacheError::Loader("failed".to_owned()))
        );
    }
    assert!(loads.load(Ordering::SeqCst) >= 1);
    assert_eq!(*cache.get_or_load(4, || Ok(9)).unwrap(), 9);
}

#[test]
fn panics_wake_the_flight_are_not_cached_and_poison_nothing() {
    let cache = Cache::<u8, u8, &'static str>::new(2, 1, 10, ManualClock::default()).unwrap();
    assert_eq!(
        cache.get_or_load(1, || -> Result<u8, &'static str> { panic!("loader panic") }),
        Err(CacheError::LoaderPanicked)
    );
    assert_eq!(*cache.get_or_load(1, || Ok(7)).unwrap(), 7);
    assert_eq!(*cache.get_or_load(2, || Ok(8)).unwrap(), 8);
}

#[test]
fn same_thread_same_key_reentrancy_is_rejected_without_blocking() {
    let cache =
        Arc::new(Cache::<String, u8, String>::new(2, 1, 10, ManualClock::default()).unwrap());
    let nested = Arc::clone(&cache);
    let value = cache
        .get_or_load("key".to_owned(), move || {
            assert_eq!(
                nested.get_or_load("key".to_owned(), || Ok(1)),
                Err(CacheError::ReentrantLoad)
            );
            Ok(2)
        })
        .unwrap();
    assert_eq!(*value, 2);
}

#[test]
fn a_blocked_loader_does_not_serialize_an_unrelated_key() {
    let cache =
        Arc::new(Cache::<String, u8, String>::new(2, 1, 10, ManualClock::default()).unwrap());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first_cache = Arc::clone(&cache);
    let first = thread::spawn(move || {
        first_cache.get_or_load("blocked".to_owned(), || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(1)
        })
    });
    entered_rx.recv_timeout(DEADLOCK_LIMIT).unwrap();

    let (done_tx, done_rx) = mpsc::channel();
    let second_cache = Arc::clone(&cache);
    let second = thread::spawn(move || {
        let result = second_cache.get_or_load("other".to_owned(), || Ok(2));
        done_tx.send(result).unwrap();
    });
    assert_eq!(
        **done_rx
            .recv_timeout(DEADLOCK_LIMIT)
            .unwrap()
            .as_ref()
            .unwrap(),
        2
    );
    release_tx.send(()).unwrap();
    assert_eq!(*first.join().unwrap().unwrap(), 1);
    second.join().unwrap();
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SameShard(&'static str);

impl Hash for SameShard {
    fn hash<H: Hasher>(&self, state: &mut H) {
        0_u8.hash(state);
    }
}

#[test]
fn shard_quota_uses_lru_hits_and_loading_entries_use_no_capacity() {
    let cache =
        Arc::new(Cache::<SameShard, u8, ()>::new(2, 1, 50, ManualClock::default()).unwrap());
    cache.get_or_load(SameShard("a"), || Ok(1)).unwrap();
    cache.get_or_load(SameShard("b"), || Ok(2)).unwrap();
    cache
        .get_or_load(SameShard("a"), || panic!("a is cached"))
        .unwrap();
    cache.get_or_load(SameShard("c"), || Ok(3)).unwrap();
    assert_eq!(cache.len(), 2);
    let reloads = AtomicUsize::new(0);
    assert_eq!(
        *cache
            .get_or_load(SameShard("b"), || {
                reloads.fetch_add(1, Ordering::SeqCst);
                Ok(22)
            })
            .unwrap(),
        22
    );
    assert_eq!(reloads.load(Ordering::SeqCst), 1);

    let one = Arc::new(Cache::<SameShard, u8, ()>::new(1, 1, 50, ManualClock::default()).unwrap());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let loading = Arc::clone(&one);
    let thread = thread::spawn(move || {
        loading.get_or_load(SameShard("loading"), || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(1)
        })
    });
    entered_rx.recv_timeout(DEADLOCK_LIMIT).unwrap();
    one.get_or_load(SameShard("ready"), || Ok(2)).unwrap();
    assert_eq!(one.len(), 1);
    release_tx.send(()).unwrap();
    thread.join().unwrap().unwrap();
    assert_eq!(one.len(), 1);
}

#[test]
fn expiry_begins_at_publication_saturates_and_zero_ttl_retains_nothing() {
    let clock = ManualClock::default();
    let cache = Cache::<u8, u8, ()>::new(2, 1, 5, clock.clone()).unwrap();
    cache
        .get_or_load(1, || {
            clock.set(100);
            Ok(1)
        })
        .unwrap();
    clock.set(104);
    cache.get_or_load(1, || panic!("still fresh")).unwrap();
    clock.set(105);
    assert_eq!(*cache.get_or_load(1, || Ok(2)).unwrap(), 2);

    clock.set(u64::MAX - 2);
    cache.get_or_load(2, || Ok(3)).unwrap();
    clock.set(u64::MAX - 1);
    cache
        .get_or_load(2, || panic!("saturated expiry remains fresh"))
        .unwrap();
    clock.set(u64::MAX);
    assert_eq!(*cache.get_or_load(2, || Ok(4)).unwrap(), 4);

    let zero = Cache::<u8, u8, ()>::new(1, 1, 0, ManualClock::default()).unwrap();
    assert_eq!(*zero.get_or_load(1, || Ok(8)).unwrap(), 8);
    assert!(zero.is_empty());
}

#[test]
fn close_wakes_flights_rejects_late_publication_and_clears_values() {
    let cache = Arc::new(Cache::<u8, u8, String>::new(2, 1, 20, ManualClock::default()).unwrap());
    cache.get_or_load(1, || Ok(1)).unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let leader_cache = Arc::clone(&cache);
    let leader = thread::spawn(move || {
        leader_cache.get_or_load(2, || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(2)
        })
    });
    entered_rx.recv_timeout(DEADLOCK_LIMIT).unwrap();

    let (waiter_tx, waiter_rx) = mpsc::channel();
    let waiter_cache = Arc::clone(&cache);
    let waiter = thread::spawn(move || {
        waiter_tx
            .send(waiter_cache.get_or_load(2, || panic!("must join the flight")))
            .unwrap();
    });
    cache.close();
    assert_eq!(
        waiter_rx.recv_timeout(DEADLOCK_LIMIT).unwrap(),
        Err(CacheError::Closed)
    );
    release_tx.send(()).unwrap();
    assert_eq!(leader.join().unwrap(), Err(CacheError::Closed));
    waiter.join().unwrap();
    assert!(cache.is_closed());
    assert!(cache.is_empty());
    assert_eq!(cache.get_or_load(3, || Ok(3)), Err(CacheError::Closed));
}
