use std::fs::{self, FileTimes, OpenOptions};
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

#[test]
fn directory_without_trailing_slash_uses_destination_state_like_rsync() {
    let root = temporary_root();
    let source = root.join("source");
    let absent_destination = root.join("renamed");
    let existing_destination = root.join("existing");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&existing_destination).unwrap();
    fs::write(source.join("data"), b"payload").unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            source.display().to_string(),
            absent_destination.display().to_string(),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::read(absent_destination.join("source/data")).unwrap(),
        b"payload"
    );

    rust_rsync_reference::run(
        parse_invocation([
            source.display().to_string(),
            existing_destination.display().to_string(),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::read(existing_destination.join("source/data")).unwrap(),
        b"payload"
    );
    clean(&root);
}

#[test]
fn multiple_sources_copy_into_an_existing_directory() {
    let root = temporary_root();
    let first = root.join("first");
    let second = root.join("second");
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    fs::write(&first, b"one").unwrap();
    fs::write(&second, b"two").unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-av".into(),
            first.display().to_string(),
            second.display().to_string(),
            destination.display().to_string(),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(destination.join("first")).unwrap(), b"one");
    assert_eq!(fs::read(destination.join("second")).unwrap(), b"two");
    clean(&root);
}

#[test]
fn ordered_filters_and_delete_protection_match_rsync_basics() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&destination).unwrap();
    fs::write(source.join("keep.txt"), b"keep").unwrap();
    fs::write(source.join("drop.tmp"), b"drop").unwrap();
    fs::write(destination.join("protected.tmp"), b"protected").unwrap();
    fs::write(destination.join("stale.txt"), b"stale").unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-a".into(),
            "--delete".into(),
            "--exclude=*.tmp".into(),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(destination.join("keep.txt")).unwrap(), b"keep");
    assert!(!destination.join("drop.tmp").exists());
    assert_eq!(
        fs::read(destination.join("protected.tmp")).unwrap(),
        b"protected"
    );
    assert!(!destination.join("stale.txt").exists());
    clean(&root);
}

#[test]
fn filter_files_are_expanded_at_their_command_line_position() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    let rules = root.join("rules.txt");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("keep.txt"), b"keep").unwrap();
    fs::write(source.join("drop.bin"), b"drop").unwrap();
    fs::write(&rules, "+ keep.txt\n- *\n").unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-a".into(),
            "--filter".into(),
            format!(". {}", rules.display()),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(destination.join("keep.txt")).unwrap(), b"keep");
    assert!(!destination.join("drop.bin").exists());
    clean(&root);
}

#[test]
fn archive_preserves_regular_file_modification_time() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).unwrap();
    let source_file = source.join("time.bin");
    fs::write(&source_file, b"time").unwrap();
    let expected = UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_123);
    OpenOptions::new()
        .write(true)
        .open(&source_file)
        .unwrap()
        .set_times(FileTimes::new().set_modified(expected))
        .unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-a".into(),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::metadata(destination.join("time.bin"))
            .unwrap()
            .modified()
            .unwrap(),
        expected
    );
    clean(&root);
}
