use std::fmt::Debug;

use rust_001::{ConfigEntry, ConfigError, ConfigSnapshot};

fn assert_required_traits<T: Debug + Clone + PartialEq + Eq>() {}

#[test]
fn public_types_keep_the_required_traits() {
    assert_required_traits::<ConfigError>();
    assert_required_traits::<ConfigEntry>();
    assert_required_traits::<ConfigSnapshot>();
}

fn snapshot_after_source_is_dropped() -> ConfigSnapshot {
    let source = String::from("Alpha=one\nBeta=two\n");
    ConfigSnapshot::parse(&source).unwrap()
}

#[test]
fn parsed_data_outlives_and_is_independent_of_the_source() {
    let snapshot = snapshot_after_source_is_dropped();
    assert_eq!(snapshot.get("alpha"), Some("one"));
    assert_eq!(snapshot.get("BETA"), Some("two"));
}

#[test]
fn set_takes_ownership_and_clones_are_independent() {
    let mut original = ConfigSnapshot::parse("Name=first\n").unwrap();
    let key = String::from("dynamic");
    let value = String::from("owned value");
    assert_eq!(original.set(key, value), None);

    let mut clone = original.clone();
    assert_eq!(
        clone.set(String::from("NAME"), String::from("second")),
        Some("first".to_owned())
    );
    assert_eq!(original.get("name"), Some("first"));
    assert_eq!(clone.get("name"), Some("second"));
    assert_eq!(original.get("dynamic"), Some("owned value"));
}

#[test]
fn preserves_order_spelling_and_first_separator_behavior() {
    let snapshot = ConfigSnapshot::parse("\tOne\t=\t1=2\t\nTwo= value \n").unwrap();
    assert_eq!(snapshot.entries().len(), 2);
    assert_eq!(snapshot.entries()[0].key(), "One");
    assert_eq!(snapshot.entries()[0].value(), "1=2");
    assert_eq!(snapshot.entries()[1].key(), "Two");
    assert_eq!(snapshot.entries()[1].value(), "value");

    let final_bare_cr = ConfigSnapshot::parse("key=value\r").unwrap();
    assert_eq!(final_bare_cr.get("key"), Some("value\r"));
}

#[test]
fn counts_ignored_physical_lines_in_errors() {
    assert_eq!(
        ConfigSnapshot::parse("\n\t\nvalid=yes\nBadLine"),
        Err(ConfigError::MissingEquals { line: 4 })
    );
    assert_eq!(ConfigSnapshot::parse(" \t \n").unwrap().entries(), &[]);
    assert_eq!(ConfigSnapshot::parse("").unwrap().entries(), &[]);
}

#[test]
fn case_folding_is_ascii_only() {
    let snapshot = ConfigSnapshot::parse("Ä=upper\nä=lower").unwrap();
    assert_eq!(snapshot.get("Ä"), Some("upper"));
    assert_eq!(snapshot.get("ä"), Some("lower"));
}
