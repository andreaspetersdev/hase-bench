use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rust_rsync::parse_invocation;

fn temporary_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hasebench-rsync-reference-{}-{nonce}",
        std::process::id()
    ))
}

fn clean(path: &Path) {
    if path.is_dir() {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn copies_contents_and_deletes_extraneous_entries() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::create_dir_all(&destination).unwrap();
    fs::write(source.join("nested/data.bin"), b"payload").unwrap();
    fs::write(destination.join("old"), b"obsolete").unwrap();

    let invocation = parse_invocation([
        "-a".into(),
        "--delete".into(),
        format!("{}/", source.display()),
        format!("{}/", destination.display()),
    ])
    .unwrap();
    rust_rsync_reference::run(invocation).unwrap();

    assert_eq!(
        fs::read(destination.join("nested/data.bin")).unwrap(),
        b"payload"
    );
    assert!(!destination.join("old").exists());
    clean(&root);
}

#[test]
fn dry_run_does_not_create_the_destination() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("data"), b"payload").unwrap();

    let invocation = parse_invocation([
        "-a".into(),
        "--dry-run".into(),
        format!("{}/", source.display()),
        format!("{}/", destination.display()),
    ])
    .unwrap();
    rust_rsync_reference::run(invocation).unwrap();

    assert!(!destination.exists());
    clean(&root);
}
