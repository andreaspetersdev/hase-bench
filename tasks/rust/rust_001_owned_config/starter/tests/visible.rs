use rust_001::{ConfigError, ConfigSnapshot};

#[test]
fn parses_and_looks_up_entries() {
    let input = " Host = example.test \r\nport= 8080\n token = a=b=c\n\n";
    let snapshot = ConfigSnapshot::parse(input).unwrap();

    assert_eq!(snapshot.get("host"), Some("example.test"));
    assert_eq!(snapshot.get("PORT"), Some("8080"));
    assert_eq!(snapshot.get("token"), Some("a=b=c"));
    assert_eq!(snapshot.entries().len(), 3);
    assert_eq!(snapshot.entries()[0].key(), "Host");
}

#[test]
fn set_updates_in_place_and_appends_new_entries() {
    let mut snapshot = ConfigSnapshot::parse("Host=old\n").unwrap();

    assert_eq!(snapshot.set("HOST", "new").as_deref(), Some("old"));
    assert_eq!(snapshot.set("port", "443").as_deref(), None);
    assert_eq!(snapshot.get("host"), Some("new"));
    assert_eq!(snapshot.entries()[0].key(), "Host");
    assert_eq!(snapshot.entries()[1].key(), "port");
}

#[test]
fn reports_line_or_duplicate_errors() {
    assert_eq!(
        ConfigSnapshot::parse("ok=1\nbroken"),
        Err(ConfigError::MissingEquals { line: 2 })
    );
    assert_eq!(
        ConfigSnapshot::parse(" =value"),
        Err(ConfigError::EmptyKey { line: 1 })
    );
    assert_eq!(
        ConfigSnapshot::parse("Name=first\nname=second"),
        Err(ConfigError::DuplicateKey {
            line: 2,
            key: "name".to_owned(),
        })
    );
}
