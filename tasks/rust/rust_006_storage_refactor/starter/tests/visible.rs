use rust_006::{DocumentService, MemoryStore, Operation, ServiceError};

#[test]
fn executes_legacy_get_put_delete_batch_and_commits_owned_values() {
    let store = MemoryStore::from_entries([
        ("alpha".to_owned(), b"old".to_vec()),
        ("remove".to_owned(), b"gone".to_vec()),
    ]);
    let observed = store.clone();
    let mut service = DocumentService::new(store);
    let mut request_value = b"new".to_vec();
    let results = service
        .execute(vec![
            Operation::Get {
                key: "alpha".to_owned(),
            },
            Operation::Put {
                key: "alpha".to_owned(),
                value: request_value.clone(),
            },
            Operation::Delete {
                key: "remove".to_owned(),
            },
        ])
        .unwrap();
    request_value[0] = b'X';

    assert_eq!(
        results,
        vec![
            Some(b"old".to_vec()),
            Some(b"old".to_vec()),
            Some(b"gone".to_vec()),
        ]
    );
    assert_eq!(observed.snapshot().get("alpha"), Some(&b"new".to_vec()));
    assert!(!observed.snapshot().contains_key("remove"));
}

#[test]
fn validates_the_complete_batch_before_mutating_legacy_storage() {
    let store = MemoryStore::from_entries([("stable".to_owned(), b"value".to_vec())]);
    let observed = store.clone();
    let mut service = DocumentService::new(store);
    let error = service
        .execute(vec![
            Operation::Put {
                key: "stable".to_owned(),
                value: b"changed".to_vec(),
            },
            Operation::Get {
                key: "not valid".to_owned(),
            },
        ])
        .unwrap_err();

    assert_eq!(
        error,
        ServiceError::InvalidKey {
            operation: 1,
            key: "not valid".to_owned(),
        }
    );
    assert_eq!(observed.snapshot().get("stable"), Some(&b"value".to_vec()));
}

#[test]
fn accepts_boundary_keys_and_empty_batches() {
    let store = MemoryStore::default();
    let observed = store.clone();
    let mut service = DocumentService::new(store);
    assert_eq!(
        service.execute(vec![]).unwrap(),
        Vec::<Option<Vec<u8>>>::new()
    );
    let key = format!("{}-_.", "a".repeat(61));
    service
        .execute(vec![Operation::Put {
            key: key.clone(),
            value: vec![],
        }])
        .unwrap();
    assert_eq!(key.len(), 64);
    assert_eq!(observed.snapshot().get(&key), Some(&vec![]));
}
