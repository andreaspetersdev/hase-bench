use std::error::Error;
use std::fmt::Debug;

use rust_003::{
    CounterStore, LookupError, OperationError, ParseOperationError, PipelineError, UpdateError,
    apply_operations,
};

fn assert_value_traits<T: Debug + Clone + PartialEq + Eq>() {}
fn assert_error<T: Error>() {}

#[test]
fn public_types_keep_required_traits_and_error_implementations() {
    assert_value_traits::<CounterStore>();
    assert_value_traits::<ParseOperationError>();
    assert_value_traits::<LookupError>();
    assert_value_traits::<UpdateError>();
    assert_value_traits::<OperationError>();
    assert_value_traits::<PipelineError>();
    assert_error::<ParseOperationError>();
    assert_error::<LookupError>();
    assert_error::<UpdateError>();
    assert_error::<OperationError>();
    assert_error::<PipelineError>();
}

#[test]
fn from_conversions_preserve_typed_errors() {
    let parse = ParseOperationError::InvalidDelta {
        value: "bad".to_owned(),
    };
    let lookup = LookupError::MissingKey {
        key: "absent".to_owned(),
    };
    let update = UpdateError::Overflow {
        key: "n".to_owned(),
        current: i64::MAX,
        delta: 1,
    };

    assert_eq!(
        OperationError::from(parse.clone()),
        OperationError::Parse(parse)
    );
    assert_eq!(
        OperationError::from(lookup.clone()),
        OperationError::Lookup(lookup)
    );
    assert_eq!(
        OperationError::from(update.clone()),
        OperationError::Update(update)
    );
}

#[test]
fn exposes_the_complete_typed_source_chain() {
    let operations = [
        OperationError::Parse(ParseOperationError::MissingEquals),
        OperationError::Lookup(LookupError::MissingKey {
            key: "ghost".to_owned(),
        }),
        OperationError::Update(UpdateError::Overflow {
            key: "max".to_owned(),
            current: i64::MAX,
            delta: 1,
        }),
    ];

    for expected in operations {
        let error = PipelineError {
            operation_index: 3,
            error: expected.clone(),
        };
        let operation = error.source().expect("pipeline source");
        assert_eq!(operation.downcast_ref::<OperationError>(), Some(&expected));
        let leaf = operation.source().expect("operation source");
        match &expected {
            OperationError::Parse(expected) => {
                assert_eq!(leaf.downcast_ref::<ParseOperationError>(), Some(expected));
            }
            OperationError::Lookup(expected) => {
                assert_eq!(leaf.downcast_ref::<LookupError>(), Some(expected));
            }
            OperationError::Update(expected) => {
                assert_eq!(leaf.downcast_ref::<UpdateError>(), Some(expected));
            }
        }
        assert!(leaf.source().is_none());
        assert!(!error.to_string().is_empty());
        assert!(!operation.to_string().is_empty());
        assert!(!leaf.to_string().is_empty());
    }
}

fn store_with_limits() -> CounterStore {
    let mut store = CounterStore::new();
    store.insert("normal", 7);
    store.insert("max", i64::MAX);
    store.insert("min", i64::MIN);
    store
}

#[test]
fn enforces_the_parse_error_precedence_matrix() {
    let cases = [
        (" no separator ", ParseOperationError::MissingEquals),
        (" \t =not-a-number", ParseOperationError::EmptyKey),
        (
            "missing=not-a-number",
            ParseOperationError::InvalidDelta {
                value: "not-a-number".to_owned(),
            },
        ),
        (
            "normal=1=2",
            ParseOperationError::InvalidDelta {
                value: "1=2".to_owned(),
            },
        ),
        (
            "normal=9223372036854775808",
            ParseOperationError::InvalidDelta {
                value: "9223372036854775808".to_owned(),
            },
        ),
    ];

    for (text, expected) in cases {
        let mut store = store_with_limits();
        assert_eq!(
            apply_operations(&mut store, &[text]),
            Err(PipelineError {
                operation_index: 0,
                error: OperationError::Parse(expected),
            })
        );
        assert_eq!(store.get("normal"), Some(7));
    }
}

#[test]
fn keys_are_case_sensitive_and_trim_only_ascii_horizontal_space() {
    let mut store = CounterStore::new();
    store.insert("Name", 2);
    store.insert("é", 4);

    assert_eq!(
        apply_operations(&mut store, &["\tName\t=\t+3\t", "é=-1"]),
        Ok(vec![5, 3])
    );
    assert_eq!(
        apply_operations(&mut store, &["name=1"]),
        Err(PipelineError {
            operation_index: 0,
            error: OperationError::Lookup(LookupError::MissingKey {
                key: "name".to_owned(),
            }),
        })
    );
    assert_eq!(
        apply_operations(&mut store, &["\nName=1"]),
        Err(PipelineError {
            operation_index: 0,
            error: OperationError::Lookup(LookupError::MissingKey {
                key: "\nName".to_owned(),
            }),
        })
    );
}

#[test]
fn stops_at_the_first_failure_and_keeps_only_prior_updates() {
    let mut store = CounterStore::new();
    store.insert("a", 1);
    store.insert("b", 10);

    assert_eq!(
        apply_operations(&mut store, &["a=2", "missing=4", "b=5"]),
        Err(PipelineError {
            operation_index: 1,
            error: OperationError::Lookup(LookupError::MissingKey {
                key: "missing".to_owned(),
            }),
        })
    );
    assert_eq!(store.get("a"), Some(3));
    assert_eq!(store.get("b"), Some(10));
}

#[test]
fn checked_overflow_preserves_both_limits() {
    let mut store = store_with_limits();
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

    assert_eq!(
        apply_operations(&mut store, &["min=-1"]),
        Err(PipelineError {
            operation_index: 0,
            error: OperationError::Update(UpdateError::Overflow {
                key: "min".to_owned(),
                current: i64::MIN,
                delta: -1,
            }),
        })
    );
    assert_eq!(store.get("min"), Some(i64::MIN));
}

#[test]
fn empty_input_succeeds_and_insert_reports_replacement() {
    let mut store = CounterStore::new();
    assert_eq!(store.insert(String::from("owned"), 1), None);
    assert_eq!(store.insert("owned", 2), Some(1));
    assert_eq!(apply_operations(&mut store, &[]), Ok(Vec::new()));
    assert_eq!(store.get("owned"), Some(2));
}
