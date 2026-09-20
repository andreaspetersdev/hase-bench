use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use rust_006::{
    DocumentService, MemoryStore, Middleware, MiddlewareError, MiddlewarePhase, Operation,
    OperationFailure, ServiceError, Storage, StorageError, Transaction,
};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap()
}

#[derive(Default)]
struct ProbeState {
    committed: BTreeMap<String, Vec<u8>>,
    events: Vec<String>,
    fail_begin: bool,
    fail_commit: bool,
    fail_rollback: bool,
}

struct ProbeStorage {
    state: Arc<Mutex<ProbeState>>,
}

impl ProbeStorage {
    fn new(state: Arc<Mutex<ProbeState>>) -> Self {
        Self { state }
    }
}

impl Storage for ProbeStorage {
    fn begin(&mut self) -> Result<Box<dyn Transaction + '_>, StorageError> {
        let mut state = lock(&self.state);
        state.events.push("begin".to_owned());
        if state.fail_begin {
            return Err(StorageError::Backend("begin".to_owned()));
        }
        let staged = state.committed.clone();
        drop(state);
        Ok(Box::new(ProbeTransaction {
            state: Arc::clone(&self.state),
            staged,
            closed: false,
        }))
    }
}

struct ProbeTransaction {
    state: Arc<Mutex<ProbeState>>,
    staged: BTreeMap<String, Vec<u8>>,
    closed: bool,
}

impl ProbeTransaction {
    fn ensure_open(&self) -> Result<(), StorageError> {
        if self.closed {
            Err(StorageError::TransactionClosed)
        } else {
            Ok(())
        }
    }
}

impl Transaction for ProbeTransaction {
    fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.ensure_open()?;
        lock(&self.state).events.push(format!("get:{key}"));
        if key == "storage_fail" {
            return Err(StorageError::Backend("get".to_owned()));
        }
        Ok(self.staged.get(key).cloned())
    }

    fn put(&mut self, key: &str, value: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        self.ensure_open()?;
        lock(&self.state).events.push(format!("put:{key}"));
        if key == "storage_fail" {
            return Err(StorageError::Conflict("put".to_owned()));
        }
        Ok(self.staged.insert(key.to_owned(), value.to_vec()))
    }

    fn delete(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.ensure_open()?;
        lock(&self.state).events.push(format!("delete:{key}"));
        if key == "storage_fail" {
            return Err(StorageError::Backend("delete".to_owned()));
        }
        Ok(self.staged.remove(key))
    }

    fn commit(&mut self) -> Result<(), StorageError> {
        self.ensure_open()?;
        let mut state = lock(&self.state);
        state.events.push("commit".to_owned());
        if state.fail_commit {
            return Err(StorageError::Conflict("commit".to_owned()));
        }
        state.committed = self.staged.clone();
        self.closed = true;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), StorageError> {
        self.ensure_open()?;
        let mut state = lock(&self.state);
        state.events.push("rollback".to_owned());
        self.closed = true;
        if state.fail_rollback {
            Err(StorageError::Backend("rollback".to_owned()))
        } else {
            Ok(())
        }
    }
}

struct ProbeMiddleware {
    name: &'static str,
    events: Arc<Mutex<Vec<String>>>,
    reject: Option<(MiddlewarePhase, usize)>,
}

impl Middleware for ProbeMiddleware {
    fn handle(
        &mut self,
        phase: MiddlewarePhase,
        operation_index: usize,
        _operation: &Operation,
    ) -> Result<(), MiddlewareError> {
        lock(&self.events).push(format!("{}:{phase:?}:{operation_index}", self.name));
        if self.reject == Some((phase, operation_index)) {
            Err(MiddlewareError::Rejected(self.name.to_owned()))
        } else {
            Ok(())
        }
    }
}

#[test]
fn public_traits_are_object_safe_and_errors_keep_value_traits() {
    fn storage_object(_: &mut dyn Storage) {}
    fn transaction_object(_: &mut dyn Transaction) {}
    fn middleware_object(_: &mut dyn Middleware) {}
    fn non_generic_service_type(_: &mut DocumentService) {}
    fn value_traits<T: std::fmt::Debug + Clone + PartialEq + Eq>() {}

    let mut service = DocumentService::new(MemoryStore::default());
    non_generic_service_type(&mut service);
    let mut store = MemoryStore::default();
    storage_object(&mut store);
    let mut transaction = store.begin().unwrap();
    transaction_object(&mut *transaction);
    struct Noop;
    impl Middleware for Noop {
        fn handle(
            &mut self,
            _phase: MiddlewarePhase,
            _operation_index: usize,
            _operation: &Operation,
        ) -> Result<(), MiddlewareError> {
            Ok(())
        }
    }
    middleware_object(&mut Noop);
    value_traits::<Operation>();
    value_traits::<StorageError>();
    value_traits::<MiddlewareError>();
    value_traits::<OperationFailure>();
    value_traits::<ServiceError>();
}

#[test]
fn independent_backend_and_middleware_observe_the_exact_success_order() {
    let storage_state = Arc::new(Mutex::new(ProbeState {
        committed: BTreeMap::from([("x".to_owned(), b"old".to_vec())]),
        ..ProbeState::default()
    }));
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let mut service = DocumentService::new(ProbeStorage::new(Arc::clone(&storage_state)));
    service.add_middleware(ProbeMiddleware {
        name: "A",
        events: Arc::clone(&hooks),
        reject: None,
    });
    service.add_middleware(ProbeMiddleware {
        name: "B",
        events: Arc::clone(&hooks),
        reject: None,
    });

    assert_eq!(
        service
            .execute(vec![
                Operation::Get {
                    key: "x".to_owned(),
                },
                Operation::Put {
                    key: "x".to_owned(),
                    value: b"new".to_vec(),
                },
            ])
            .unwrap(),
        vec![Some(b"old".to_vec()), Some(b"old".to_vec())]
    );
    assert_eq!(
        *lock(&hooks),
        vec![
            "A:Before:0",
            "B:Before:0",
            "B:After:0",
            "A:After:0",
            "A:Before:1",
            "B:Before:1",
            "B:After:1",
            "A:After:1",
        ]
    );
    let state = lock(&storage_state);
    assert_eq!(state.events, ["begin", "get:x", "put:x", "commit"]);
    assert_eq!(state.committed.get("x"), Some(&b"new".to_vec()));
}

#[test]
fn validates_all_keys_before_storage_or_middleware_and_owns_the_error_key() {
    let storage_state = Arc::new(Mutex::new(ProbeState::default()));
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let mut service = DocumentService::new(ProbeStorage::new(Arc::clone(&storage_state)));
    service.add_middleware(ProbeMiddleware {
        name: "never",
        events: Arc::clone(&hooks),
        reject: None,
    });
    let invalid = "é".to_owned();
    assert_eq!(
        service.execute(vec![
            Operation::Put {
                key: "valid".to_owned(),
                value: vec![1],
            },
            Operation::Get {
                key: invalid.clone(),
            },
            Operation::Get {
                key: "x".repeat(65),
            },
        ]),
        Err(ServiceError::InvalidKey {
            operation: 1,
            key: invalid,
        })
    );
    assert!(lock(&storage_state).events.is_empty());
    assert!(lock(&hooks).is_empty());
}

#[test]
fn empty_batch_does_not_begin_or_call_middleware() {
    let storage_state = Arc::new(Mutex::new(ProbeState::default()));
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let mut service = DocumentService::new(ProbeStorage::new(Arc::clone(&storage_state)));
    service.add_middleware(ProbeMiddleware {
        name: "never",
        events: Arc::clone(&hooks),
        reject: Some((MiddlewarePhase::Before, 0)),
    });
    assert_eq!(service.execute(vec![]), Ok(vec![]));
    assert!(lock(&storage_state).events.is_empty());
    assert!(lock(&hooks).is_empty());
}

#[test]
fn before_middleware_failure_stops_and_rolls_back_once() {
    let storage_state = Arc::new(Mutex::new(ProbeState {
        committed: BTreeMap::from([("x".to_owned(), b"old".to_vec())]),
        ..ProbeState::default()
    }));
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let mut service = DocumentService::new(ProbeStorage::new(Arc::clone(&storage_state)));
    service.add_middleware(ProbeMiddleware {
        name: "A",
        events: Arc::clone(&hooks),
        reject: Some((MiddlewarePhase::Before, 1)),
    });
    service.add_middleware(ProbeMiddleware {
        name: "B",
        events: Arc::clone(&hooks),
        reject: None,
    });
    let error = service
        .execute(vec![
            Operation::Put {
                key: "x".to_owned(),
                value: b"changed".to_vec(),
            },
            Operation::Delete {
                key: "x".to_owned(),
            },
        ])
        .unwrap_err();
    assert_eq!(
        error,
        ServiceError::Operation {
            operation: 1,
            source: OperationFailure::Middleware {
                phase: MiddlewarePhase::Before,
                source: MiddlewareError::Rejected("A".to_owned()),
            },
        }
    );
    assert_eq!(
        *lock(&hooks),
        [
            "A:Before:0",
            "B:Before:0",
            "B:After:0",
            "A:After:0",
            "A:Before:1",
        ]
    );
    let state = lock(&storage_state);
    assert_eq!(state.events, ["begin", "put:x", "rollback"]);
    assert_eq!(state.committed.get("x"), Some(&b"old".to_vec()));
}

#[test]
fn reverse_after_failure_rolls_back_and_skips_later_operations() {
    let storage_state = Arc::new(Mutex::new(ProbeState::default()));
    let hooks = Arc::new(Mutex::new(Vec::new()));
    let mut service = DocumentService::new(ProbeStorage::new(Arc::clone(&storage_state)));
    service.add_middleware(ProbeMiddleware {
        name: "A",
        events: Arc::clone(&hooks),
        reject: Some((MiddlewarePhase::After, 0)),
    });
    service.add_middleware(ProbeMiddleware {
        name: "B",
        events: Arc::clone(&hooks),
        reject: None,
    });
    assert!(matches!(
        service.execute(vec![
            Operation::Put {
                key: "x".to_owned(),
                value: vec![1],
            },
            Operation::Get {
                key: "x".to_owned(),
            },
        ]),
        Err(ServiceError::Operation {
            operation: 0,
            source: OperationFailure::Middleware {
                phase: MiddlewarePhase::After,
                ..
            }
        })
    ));
    assert_eq!(
        *lock(&hooks),
        ["A:Before:0", "B:Before:0", "B:After:0", "A:After:0"]
    );
    let state = lock(&storage_state);
    assert_eq!(state.events, ["begin", "put:x", "rollback"]);
    assert!(!state.committed.contains_key("x"));
}

#[test]
fn storage_operation_failure_is_typed_and_rolled_back() {
    let state = Arc::new(Mutex::new(ProbeState::default()));
    let mut service = DocumentService::new(ProbeStorage::new(Arc::clone(&state)));
    assert_eq!(
        service.execute(vec![Operation::Put {
            key: "storage_fail".to_owned(),
            value: vec![1],
        }]),
        Err(ServiceError::Operation {
            operation: 0,
            source: OperationFailure::Storage(StorageError::Conflict("put".to_owned())),
        })
    );
    assert_eq!(
        lock(&state).events,
        ["begin", "put:storage_fail", "rollback"]
    );
}

#[test]
fn begin_commit_and_rollback_failures_preserve_typed_context() {
    let begin = Arc::new(Mutex::new(ProbeState {
        fail_begin: true,
        ..ProbeState::default()
    }));
    let mut begin_service = DocumentService::new(ProbeStorage::new(begin));
    assert_eq!(
        begin_service.execute(vec![Operation::Get {
            key: "x".to_owned(),
        }]),
        Err(ServiceError::Begin(StorageError::Backend(
            "begin".to_owned()
        )))
    );

    let commit = Arc::new(Mutex::new(ProbeState {
        fail_commit: true,
        ..ProbeState::default()
    }));
    let mut commit_service = DocumentService::new(ProbeStorage::new(Arc::clone(&commit)));
    assert_eq!(
        commit_service.execute(vec![Operation::Put {
            key: "x".to_owned(),
            value: vec![1],
        }]),
        Err(ServiceError::Commit(StorageError::Conflict(
            "commit".to_owned()
        )))
    );
    assert_eq!(
        lock(&commit).events,
        ["begin", "put:x", "commit", "rollback"]
    );

    let commit_and_rollback = Arc::new(Mutex::new(ProbeState {
        fail_commit: true,
        fail_rollback: true,
        ..ProbeState::default()
    }));
    let mut doubly_failed =
        DocumentService::new(ProbeStorage::new(Arc::clone(&commit_and_rollback)));
    assert_eq!(
        doubly_failed.execute(vec![Operation::Get {
            key: "x".to_owned(),
        }]),
        Err(ServiceError::Rollback {
            original: Box::new(ServiceError::Commit(StorageError::Conflict(
                "commit".to_owned(),
            ))),
            rollback: StorageError::Backend("rollback".to_owned()),
        })
    );
    assert_eq!(
        lock(&commit_and_rollback).events,
        ["begin", "get:x", "commit", "rollback"]
    );

    let rollback = Arc::new(Mutex::new(ProbeState {
        fail_rollback: true,
        ..ProbeState::default()
    }));
    let mut rollback_service = DocumentService::new(ProbeStorage::new(rollback));
    assert_eq!(
        rollback_service.execute(vec![Operation::Get {
            key: "storage_fail".to_owned(),
        }]),
        Err(ServiceError::Rollback {
            original: Box::new(ServiceError::Operation {
                operation: 0,
                source: OperationFailure::Storage(StorageError::Backend("get".to_owned())),
            }),
            rollback: StorageError::Backend("rollback".to_owned()),
        })
    );
}

#[test]
fn memory_transactions_are_isolated_and_close_permanently() {
    let mut store = MemoryStore::from_entries([("x".to_owned(), b"old".to_vec())]);
    let observed = store.clone();
    {
        let mut transaction = store.begin().unwrap();
        assert_eq!(transaction.put("x", b"new").unwrap(), Some(b"old".to_vec()));
        assert_eq!(observed.snapshot().get("x"), Some(&b"old".to_vec()));
        transaction.rollback().unwrap();
        assert_eq!(transaction.get("x"), Err(StorageError::TransactionClosed));
        assert_eq!(transaction.commit(), Err(StorageError::TransactionClosed));
    }
    {
        let mut transaction = store.begin().unwrap();
        transaction.put("x", b"new").unwrap();
        transaction.commit().unwrap();
        assert_eq!(transaction.rollback(), Err(StorageError::TransactionClosed));
    }
    assert_eq!(observed.snapshot().get("x"), Some(&b"new".to_vec()));
}
