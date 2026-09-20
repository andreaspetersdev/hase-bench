use std::fs::{self, FileTimes, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::process::Command;

use rust_rsync::fs::{CapabilityStatus, MetadataFeature};
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

#[allow(clippy::permissions_set_readonly_false)]
fn make_writable(path: &Path) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).unwrap();
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

#[test]
fn directory_merge_rules_are_anchored_and_inherited() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join(".rules"), "- *.tmp\n").unwrap();
    fs::write(source.join("root.tmp"), b"excluded").unwrap();
    fs::write(source.join("root.txt"), b"root").unwrap();
    fs::write(source.join("nested/.rules"), "- /private.txt\n").unwrap();
    fs::write(source.join("nested/private.txt"), b"private").unwrap();
    fs::write(source.join("nested/public.txt"), b"public").unwrap();
    fs::write(source.join("nested/child.tmp"), b"excluded").unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-a".into(),
            "--filter".into(),
            "dir-merge .rules".into(),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert!(destination.join("root.txt").exists());
    assert!(!destination.join("root.tmp").exists());
    assert!(destination.join("nested/public.txt").exists());
    assert!(!destination.join("nested/private.txt").exists());
    assert!(!destination.join("nested/child.tmp").exists());
    clean(&root);
}

#[test]
fn hard_link_groups_are_preserved_when_requested() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("first"), b"shared").unwrap();
    fs::hard_link(source.join("first"), source.join("second")).unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-aH".into(),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    fs::write(destination.join("first"), b"changed").unwrap();
    assert_eq!(fs::read(destination.join("second")).unwrap(), b"changed");
    clean(&root);
}

#[test]
fn archive_preserves_windows_read_only_mapping() {
    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).unwrap();
    let source_file = source.join("readonly");
    fs::write(&source_file, b"readonly").unwrap();
    let mut permissions = fs::metadata(&source_file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&source_file, permissions).unwrap();

    rust_rsync_reference::run(
        parse_invocation([
            "-a".into(),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert!(
        fs::metadata(destination.join("readonly"))
            .unwrap()
            .permissions()
            .readonly()
    );
    make_writable(&source_file);
    make_writable(&destination.join("readonly"));
    clean(&root);
}

#[test]
fn link_options_select_object_referent_and_safe_policy_when_supported() {
    let root = temporary_root();
    let source = root.join("source");
    let outside = root.join("outside.txt");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("inside.txt"), b"inside").unwrap();
    fs::write(&outside, b"outside").unwrap();
    if !create_file_symlink("inside.txt", &source.join("safe-link")) {
        clean(&root);
        return;
    }
    assert!(create_file_symlink(
        "../outside.txt",
        &source.join("unsafe-link")
    ));

    let preserved = root.join("preserved");
    rust_rsync_reference::run(
        parse_invocation([
            "-a".into(),
            "--safe-links".into(),
            format!("{}/", source.display()),
            format!("{}/", preserved.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert!(
        fs::symlink_metadata(preserved.join("safe-link"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!preserved.join("unsafe-link").exists());

    let followed = root.join("followed");
    rust_rsync_reference::run(
        parse_invocation([
            "-aL".into(),
            format!("{}/", source.display()),
            format!("{}/", followed.display()),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fs::read(followed.join("safe-link")).unwrap(), b"inside");
    assert_eq!(fs::read(followed.join("unsafe-link")).unwrap(), b"outside");
    assert!(
        !fs::symlink_metadata(followed.join("safe-link"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    clean(&root);
}

#[test]
fn link_policy_options_are_parsed_without_host_capability() {
    let invocation = parse_invocation(["-aHL", "--safe-links", "source/", "destination/"])
        .expect("link policy options should parse");
    assert!(invocation.options.preserve_symlinks);
    assert!(invocation.options.preserve_hard_links);
    assert!(invocation.options.copy_link_referents);
    assert!(invocation.options.safe_links);
}

#[cfg(unix)]
fn create_file_symlink(target: &str, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn create_file_symlink(target: &str, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[test]
fn metadata_capability_report_distinguishes_host_and_adapter_limits() {
    let report = rust_rsync_reference::metadata_capabilities();
    assert_eq!(
        report.status(MetadataFeature::ModificationTimes),
        Some(&CapabilityStatus::Supported)
    );
    assert_eq!(
        report.status(MetadataFeature::Permissions),
        Some(&CapabilityStatus::Supported)
    );
    assert_eq!(
        report.status(MetadataFeature::HardLinks),
        Some(&CapabilityStatus::Supported)
    );
    assert_eq!(
        report.status(MetadataFeature::Symlinks),
        Some(&CapabilityStatus::ProbeRequired)
    );
    #[cfg(windows)]
    for feature in [
        MetadataFeature::Acls,
        MetadataFeature::ExtendedAttributes,
        MetadataFeature::SparseFiles,
    ] {
        assert_eq!(report.status(feature), Some(&CapabilityStatus::Supported));
    }
    #[cfg(windows)]
    assert!(matches!(
        report.status(MetadataFeature::Ownership),
        Some(CapabilityStatus::HostUnsupported { .. })
    ));
}

#[test]
fn unavailable_metadata_requests_fail_before_destination_mutation() {
    let root = temporary_root();
    let source = root.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("data"), b"payload").unwrap();
    for (index, (flag, diagnostic, feature)) in [
        ("-o", "ownership preservation", MetadataFeature::Ownership),
        ("-g", "ownership preservation", MetadataFeature::Ownership),
        ("-A", "ACL preservation", MetadataFeature::Acls),
        (
            "-X",
            "extended-attribute preservation",
            MetadataFeature::ExtendedAttributes,
        ),
        (
            "-S",
            "sparse-file preservation",
            MetadataFeature::SparseFiles,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        if matches!(
            rust_rsync_reference::metadata_capabilities().status(feature),
            Some(CapabilityStatus::Supported)
        ) {
            continue;
        }
        let destination = root.join(format!("destination-{index}"));
        let error = rust_rsync_reference::run(
            parse_invocation([
                flag.into(),
                format!("{}/", source.display()),
                format!("{}/", destination.display()),
            ])
            .unwrap(),
        )
        .unwrap_err();
        assert_eq!(error.exit_code(), 23);
        assert!(error.to_string().contains(diagnostic));
        assert!(!destination.exists());
    }
    clean(&root);
}

#[cfg(windows)]
#[test]
fn windows_acl_named_stream_and_sparse_adapters_preserve_metadata() {
    const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;

    let root = temporary_root();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source).unwrap();
    let source_file = source.join("metadata.bin");
    let mut contents = vec![0_u8; 3 * 64 * 1024];
    contents[0] = 1;
    *contents.last_mut().unwrap() = 2;
    fs::write(&source_file, &contents).unwrap();
    fs::write(named_stream(&source_file, "hasebench"), b"opaque\x00stream").unwrap();

    let acl_result = Command::new("icacls.exe")
        .arg(&source_file)
        .arg("/inheritance:d")
        .output()
        .unwrap();
    assert!(acl_result.status.success());
    let source_sddl = canonical_acl_sddl(&source_file);

    rust_rsync_reference::run(
        parse_invocation([
            "-aAXS".into(),
            format!("{}/", source.display()),
            format!("{}/", destination.display()),
        ])
        .unwrap(),
    )
    .unwrap();

    let destination_file = destination.join("metadata.bin");
    assert_eq!(fs::read(&destination_file).unwrap(), contents);
    assert_eq!(
        fs::read(named_stream(&destination_file, "hasebench")).unwrap(),
        b"opaque\x00stream"
    );
    assert_eq!(canonical_acl_sddl(&destination_file), source_sddl);
    assert_ne!(
        fs::metadata(&destination_file).unwrap().file_attributes() & FILE_ATTRIBUTE_SPARSE_FILE,
        0
    );
    assert!(allocated_sparse_bytes(&destination_file) < contents.len() as u64);
    clean(&root);
}

#[cfg(windows)]
fn named_stream(path: &Path, name: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(":");
    value.push(name);
    PathBuf::from(value)
}

#[cfg(windows)]
fn acl_sddl(path: &Path) -> String {
    const SCRIPT: &str =
        "$ErrorActionPreference='Stop'; (Get-Acl -LiteralPath $env:HASEBENCH_ACL_PATH).Sddl";
    let output = Command::new("pwsh.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            SCRIPT,
        ])
        .env("HASEBENCH_ACL_PATH", path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[cfg(windows)]
fn canonical_acl_sddl(path: &Path) -> (String, Vec<String>) {
    let sddl = acl_sddl(path);
    let Some(first_ace) = sddl.find('(') else {
        return (sddl, Vec::new());
    };
    let header = sddl[..first_ace].to_owned();
    let mut aces = sddl[first_ace..]
        .split_inclusive(')')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    aces.sort();
    (header, aces)
}

#[cfg(windows)]
fn allocated_sparse_bytes(path: &Path) -> u64 {
    let output = Command::new("fsutil.exe")
        .args(["sparse", "queryrange"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let values = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|token| token.strip_prefix("0x"))
        .map(|value| u64::from_str_radix(value, 16).unwrap())
        .collect::<Vec<_>>();
    assert!(!values.is_empty() && values.len() % 2 == 0);
    values.iter().skip(1).step_by(2).sum()
}
