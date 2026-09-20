use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rust_rsync::parse_invocation;
use rust_rsync_reference::recovery::{RecoveryError, RecoveryPolicy, write_with_recovery};

fn temporary_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hasebench-rsync-recovery-{}-{nonce}",
        std::process::id()
    ))
}

fn clean(path: &Path) {
    if path.is_dir() {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn partial_options_select_retention_and_directory() {
    let direct = parse_invocation(["--partial", "source", "destination"]).unwrap();
    assert!(direct.options.partial);
    assert_eq!(direct.options.partial_dir, None);

    let isolated = parse_invocation(["--partial-dir=.partials", "source", "destination"]).unwrap();
    assert!(isolated.options.partial);
    assert_eq!(
        isolated.options.partial_dir,
        Some(PathBuf::from(".partials"))
    );
}

#[test]
fn partial_retains_interrupted_output_and_reuses_its_prefix() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();
    let destination = root.join("payload.bin");
    let contents = (0_u32..8192).flat_map(u32::to_le_bytes).collect::<Vec<_>>();
    let policy = RecoveryPolicy {
        retain_partial: true,
        partial_dir: None,
    };
    let error = write_with_recovery(&destination, &contents, &policy, Some(7001)).unwrap_err();
    assert!(matches!(
        error,
        RecoveryError::Interrupted {
            completed_bytes: 7001,
            retained_path: Some(ref path),
        } if path == &destination
    ));
    assert_eq!(fs::read(&destination).unwrap(), contents[..7001]);

    let result = write_with_recovery(&destination, &contents, &policy, None).unwrap();
    assert_eq!(result.reused_bytes, 7001);
    assert_eq!(fs::read(&destination).unwrap(), contents);
    clean(&root);
}

#[test]
fn partial_directory_isolates_interruption_and_commits_retry() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();
    let destination = root.join("payload.bin");
    fs::write(&destination, b"old destination").unwrap();
    let contents = vec![0x5a; 24 * 1024];
    let policy = RecoveryPolicy {
        retain_partial: true,
        partial_dir: Some(PathBuf::from(".partials")),
    };
    let partial = root.join(".partials/payload.bin");
    let error = write_with_recovery(&destination, &contents, &policy, Some(4096)).unwrap_err();
    assert!(matches!(
        error,
        RecoveryError::Interrupted {
            completed_bytes: 4096,
            retained_path: Some(ref path),
        } if path == &partial
    ));
    assert_eq!(fs::read(&destination).unwrap(), b"old destination");
    assert_eq!(fs::metadata(&partial).unwrap().len(), 4096);

    let result = write_with_recovery(&destination, &contents, &policy, None).unwrap();
    assert_eq!(result.reused_bytes, 4096);
    assert_eq!(fs::read(&destination).unwrap(), contents);
    assert!(!partial.exists());
    clean(&root);
}

#[test]
fn discarded_and_unsafe_partials_never_escape_or_survive() {
    let root = temporary_root();
    fs::create_dir_all(&root).unwrap();
    let destination = root.join("payload.bin");
    let contents = b"new payload";
    let discard = RecoveryPolicy::default();
    let error = write_with_recovery(&destination, contents, &discard, Some(3)).unwrap_err();
    assert!(matches!(
        error,
        RecoveryError::Interrupted {
            retained_path: None,
            ..
        }
    ));
    assert!(!destination.exists());
    assert!(!root.join("payload.bin.rust-rsync-partial").exists());

    let unsafe_policy = RecoveryPolicy {
        retain_partial: true,
        partial_dir: Some(PathBuf::from("../escape")),
    };
    assert!(matches!(
        write_with_recovery(&destination, contents, &unsafe_policy, None),
        Err(RecoveryError::InvalidPartialDirectory(_))
    ));
    assert!(!destination.exists());
    clean(&root);
}

#[test]
fn mismatched_retained_prefix_is_discarded_before_retry() {
    let root = temporary_root();
    let partial_dir = root.join(".partials");
    fs::create_dir_all(&partial_dir).unwrap();
    let destination = root.join("payload.bin");
    fs::write(partial_dir.join("payload.bin"), b"wrong prefix").unwrap();
    let contents = b"correct replacement payload";
    let policy = RecoveryPolicy {
        retain_partial: true,
        partial_dir: Some(PathBuf::from(".partials")),
    };
    let result = write_with_recovery(&destination, contents, &policy, None).unwrap();
    assert_eq!(result.reused_bytes, 0);
    assert_eq!(fs::read(&destination).unwrap(), contents);
    clean(&root);
}
