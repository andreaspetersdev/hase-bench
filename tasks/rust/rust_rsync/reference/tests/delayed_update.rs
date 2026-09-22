use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rust_rsync::parse_invocation;
use rust_rsync_reference::delayed_update::{
    DelayedUpdate, DelayedUpdateError, DelayedUpdateFault, apply_delayed_updates,
};

fn temporary_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hasebench-rsync-delayed-{}-{nonce}",
        std::process::id()
    ))
}

fn update(path: &str, contents: &[u8]) -> DelayedUpdate {
    DelayedUpdate {
        relative_path: PathBuf::from(path),
        contents: contents.to_vec(),
    }
}

fn transaction_entries(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".rust-rsync-delay-")
        })
        .collect()
}

#[test]
fn delay_updates_option_is_parsed() {
    let invocation = parse_invocation(["--delay-updates", "source", "destination"]).unwrap();
    assert!(invocation.options.delay_updates);
}

#[test]
fn completed_files_are_committed_as_a_final_set() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("existing.txt"), b"old").unwrap();
    let updates = [
        update("existing.txt", b"replacement"),
        update("nested/new.bin", b"new file"),
    ];

    let result = apply_delayed_updates(&root, &updates, DelayedUpdateFault::default()).unwrap();

    assert_eq!(result.committed_files, 2);
    assert_eq!(fs::read(root.join("existing.txt")).unwrap(), b"replacement");
    assert_eq!(fs::read(root.join("nested/new.bin")).unwrap(), b"new file");
    assert!(transaction_entries(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn staging_interruption_leaves_every_destination_unchanged() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("first.txt"), b"first old").unwrap();
    fs::write(root.join("second.txt"), b"second old").unwrap();
    let updates = [
        update("first.txt", b"first new"),
        update("second.txt", b"second new"),
    ];
    let fault = DelayedUpdateFault {
        interrupt_after_staged: Some(2),
        fail_before_commit: None,
    };

    assert!(matches!(
        apply_delayed_updates(&root, &updates, fault),
        Err(DelayedUpdateError::Interrupted { staged_files: 2 })
    ));
    assert_eq!(fs::read(root.join("first.txt")).unwrap(), b"first old");
    assert_eq!(fs::read(root.join("second.txt")).unwrap(), b"second old");
    assert!(transaction_entries(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn commit_failure_rolls_back_replacements_and_new_files() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("first.txt"), b"first old").unwrap();
    fs::write(root.join("second.txt"), b"second old").unwrap();
    let updates = [
        update("first.txt", b"first new"),
        update("created/first.txt", b"temporarily installed"),
        update("second.txt", b"second new"),
    ];
    let fault = DelayedUpdateFault {
        interrupt_after_staged: None,
        fail_before_commit: Some(2),
    };

    assert!(matches!(
        apply_delayed_updates(&root, &updates, fault),
        Err(DelayedUpdateError::Commit {
            rollback_error: None,
            ..
        })
    ));
    assert!(!root.join("created").exists());
    assert_eq!(fs::read(root.join("first.txt")).unwrap(), b"first old");
    assert_eq!(fs::read(root.join("second.txt")).unwrap(), b"second old");
    assert!(transaction_entries(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_or_duplicate_destinations_fail_before_staging() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();

    assert!(matches!(
        apply_delayed_updates(
            &root,
            &[update("../escape", b"bad")],
            DelayedUpdateFault::default()
        ),
        Err(DelayedUpdateError::InvalidRelativePath(_))
    ));
    assert!(matches!(
        apply_delayed_updates(
            &root,
            &[update("same", b"one"), update("same", b"two")],
            DelayedUpdateFault::default()
        ),
        Err(DelayedUpdateError::DuplicateDestination(_))
    ));
    assert!(transaction_entries(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}
