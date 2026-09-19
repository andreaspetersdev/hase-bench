use rust_003::{
    CounterStore, LookupError, OperationError, ParseOperationError, PipelineError, UpdateError,
    apply_operations,
};

fn sample_store() -> CounterStore {
    let mut store = CounterStore::new();
    store.insert("apples", 10);
    store.insert("visits", -2);
    store
}

#[test]
fn applies_operations_in_order_and_returns_new_values() {
    let mut store = sample_store();
    let values = apply_operations(&mut store, &[" apples = 5 ", "visits=-3", "apples=-2"]).unwrap();

    assert_eq!(values, vec![15, -5, 13]);
    assert_eq!(store.get("apples"), Some(13));
    assert_eq!(store.get("visits"), Some(-5));
}

#[test]
fn reports_parse_and_lookup_errors_with_the_operation_index() {
    let mut store = sample_store();
    assert_eq!(
        apply_operations(&mut store, &["apples=1", "missing"]),
        Err(PipelineError {
            operation_index: 1,
            error: OperationError::Parse(ParseOperationError::MissingEquals),
        })
    );
    assert_eq!(store.get("apples"), Some(11));

    assert_eq!(
        apply_operations(&mut store, &["unknown=4"]),
        Err(PipelineError {
            operation_index: 0,
            error: OperationError::Lookup(LookupError::MissingKey {
                key: "unknown".to_owned(),
            }),
        })
    );
}

#[test]
fn rejects_overflow_without_changing_the_counter() {
    let mut store = CounterStore::new();
    store.insert("max", i64::MAX);
    assert_eq!(
        apply_operations(&mut store, &["max=1"]),
        Err(PipelineError {
            operation_index: 0,
            error: OperationError::Update(UpdateError::Overflow {
                key: "max".to_owned(),
                current: i64::MAX,
                delta: 1,
            }),
        })
    );
    assert_eq!(store.get("max"), Some(i64::MAX));
}
