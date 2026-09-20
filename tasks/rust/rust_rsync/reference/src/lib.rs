#![forbid(unsafe_code)]

use std::collections::BTreeSet;
#[cfg(windows)]
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, FileTimes};
#[cfg(windows)]
use std::io::Read;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

use rust_rsync::config::{FilterDirective, FilterFileKind, FilterRule};
use rust_rsync::fs::{CapabilityOutcome, CapabilityReport, CapabilityStatus, MetadataFeature};
use rust_rsync::{Endpoint, Invocation, Options, PathSpec};
use same_file::Handle;

pub mod delta;

#[derive(Debug, Clone)]
struct ActiveRule {
    rule: FilterRule,
    base: PathBuf,
}

#[derive(Debug, Default)]
struct FilterProgram {
    global_rules: Vec<ActiveRule>,
    dir_merge_files: Vec<PathBuf>,
}

#[derive(Debug, Default)]
struct CopyContext {
    hard_links: Vec<(Handle, PathBuf)>,
}

#[derive(Debug)]
pub enum ReferenceError {
    UnsupportedMode,
    UnsupportedCapability {
        feature: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    UnavailableCapability {
        feature: MetadataFeature,
        reason: String,
    },
    InvalidDestination,
    InvalidFilter(String),
    MissingFileName(PathBuf),
    Io {
        path: PathBuf,
        source: io::Error,
    },
}

impl ReferenceError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnsupportedMode
            | Self::InvalidDestination
            | Self::InvalidFilter(_)
            | Self::MissingFileName(_) => 2,
            Self::Io { .. }
            | Self::UnsupportedCapability { .. }
            | Self::UnavailableCapability { .. } => 23,
        }
    }
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMode => {
                formatter.write_str("reference phase supports local endpoints only")
            }
            Self::UnsupportedCapability {
                feature,
                path,
                source,
            } => write!(
                formatter,
                "unsupported {feature} capability at {}: {source}",
                path.display()
            ),
            Self::UnavailableCapability { feature, reason } => {
                write!(
                    formatter,
                    "unsupported {} capability: {reason}",
                    feature_name(*feature)
                )
            }
            Self::InvalidDestination => {
                formatter.write_str("multiple sources require a directory destination")
            }
            Self::InvalidFilter(rule) => write!(formatter, "invalid filter rule: {rule}"),
            Self::MissingFileName(path) => {
                write!(formatter, "source has no file name: {}", path.display())
            }
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for ReferenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::UnsupportedCapability { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn run(invocation: Invocation) -> Result<(), ReferenceError> {
    let Invocation {
        options,
        sources,
        destination,
    } = invocation;
    let Endpoint::Local(destination) = destination else {
        return Err(ReferenceError::UnsupportedMode);
    };
    let sources = sources
        .into_iter()
        .map(|source| match source {
            Endpoint::Local(path) => Ok(path),
            _ => Err(ReferenceError::UnsupportedMode),
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_requested_capabilities(&options)?;
    let filters = expand_filters(&options)?;
    let mut context = CopyContext::default();
    if options.dry_run {
        return Ok(());
    }
    if sources.len() > 1 && !destination.path.is_dir() {
        return Err(ReferenceError::InvalidDestination);
    }
    for source in &sources {
        transfer_source(source, &destination.path, &options, &filters, &mut context)?;
    }
    Ok(())
}

pub fn metadata_capabilities() -> CapabilityReport {
    let ownership = if cfg!(windows) {
        CapabilityStatus::HostUnsupported {
            reason: "no implicit Windows SID to Unix uid/gid mapping".to_owned(),
        }
    } else {
        CapabilityStatus::AdapterUnavailable {
            reason: "owner/group preservation is not implemented in the Phase 0 reference"
                .to_owned(),
        }
    };
    CapabilityReport {
        outcomes: vec![
            supported(MetadataFeature::ModificationTimes),
            supported(MetadataFeature::Permissions),
            CapabilityOutcome {
                feature: MetadataFeature::Ownership,
                status: ownership,
            },
            windows_adapter(
                MetadataFeature::Acls,
                "Unix ACL preservation is not implemented in the Phase 0 reference",
            ),
            windows_adapter(
                MetadataFeature::ExtendedAttributes,
                "Unix xattr preservation is not implemented in the Phase 0 reference",
            ),
            CapabilityOutcome {
                feature: MetadataFeature::Symlinks,
                status: CapabilityStatus::ProbeRequired,
            },
            supported(MetadataFeature::HardLinks),
            windows_adapter(
                MetadataFeature::SparseFiles,
                "Unix sparse allocation is not implemented in the Phase 0 reference",
            ),
        ],
    }
}

fn supported(feature: MetadataFeature) -> CapabilityOutcome {
    CapabilityOutcome {
        feature,
        status: CapabilityStatus::Supported,
    }
}

fn adapter_unavailable(feature: MetadataFeature, reason: &str) -> CapabilityOutcome {
    CapabilityOutcome {
        feature,
        status: CapabilityStatus::AdapterUnavailable {
            reason: reason.to_owned(),
        },
    }
}

fn windows_adapter(feature: MetadataFeature, non_windows_reason: &str) -> CapabilityOutcome {
    if cfg!(windows) {
        supported(feature)
    } else {
        adapter_unavailable(feature, non_windows_reason)
    }
}

fn validate_requested_capabilities(options: &Options) -> Result<(), ReferenceError> {
    let report = metadata_capabilities();
    let requested = [
        (
            options.preserve_owner || options.preserve_group,
            MetadataFeature::Ownership,
        ),
        (options.preserve_acls, MetadataFeature::Acls),
        (options.preserve_xattrs, MetadataFeature::ExtendedAttributes),
        (options.sparse, MetadataFeature::SparseFiles),
    ];
    for (is_requested, feature) in requested {
        if !is_requested {
            continue;
        }
        match report.status(feature) {
            Some(CapabilityStatus::Supported | CapabilityStatus::ProbeRequired) => {}
            Some(
                CapabilityStatus::HostUnsupported { reason }
                | CapabilityStatus::AdapterUnavailable { reason },
            ) => {
                return Err(ReferenceError::UnavailableCapability {
                    feature,
                    reason: reason.clone(),
                });
            }
            None => {
                return Err(ReferenceError::UnavailableCapability {
                    feature,
                    reason: "adapter did not report this capability".to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn feature_name(feature: MetadataFeature) -> &'static str {
    match feature {
        MetadataFeature::ModificationTimes => "modification-time preservation",
        MetadataFeature::Permissions => "permission preservation",
        MetadataFeature::Ownership => "ownership preservation",
        MetadataFeature::Acls => "ACL preservation",
        MetadataFeature::ExtendedAttributes => "extended-attribute preservation",
        MetadataFeature::Symlinks => "symbolic-link preservation",
        MetadataFeature::HardLinks => "hard-link preservation",
        MetadataFeature::SparseFiles => "sparse-file preservation",
    }
}

fn transfer_source(
    source: &PathSpec,
    destination: &Path,
    options: &Options,
    filters: &FilterProgram,
    context: &mut CopyContext,
) -> Result<(), ReferenceError> {
    let metadata =
        fs::symlink_metadata(&source.path).map_err(|error| io_error(&source.path, error))?;
    if metadata.is_dir() && source.copy_contents {
        create_directory(destination)?;
        sync_directory(
            &source.path,
            destination,
            Path::new(""),
            options,
            filters,
            &filters.global_rules,
            context,
        )
    } else {
        let name = source
            .path
            .file_name()
            .ok_or_else(|| ReferenceError::MissingFileName(source.path.clone()))?;
        let target = if destination.is_dir() || metadata.is_dir() {
            destination.join(name)
        } else {
            destination.to_path_buf()
        };
        copy_entry(
            &source.path,
            &target,
            Path::new(name),
            options,
            filters,
            &filters.global_rules,
            context,
        )
    }
}

fn sync_directory(
    source: &Path,
    destination: &Path,
    relative: &Path,
    options: &Options,
    program: &FilterProgram,
    inherited_rules: &[ActiveRule],
    context: &mut CopyContext,
) -> Result<(), ReferenceError> {
    create_directory(destination)?;
    let mut filters = inherited_rules.to_vec();
    for file in &program.dir_merge_files {
        let path = source.join(file);
        if path.is_file() {
            filters.extend(read_filter_file(&path, FilterFileKind::Merge, relative)?);
        }
    }
    let mut retained = BTreeSet::new();
    let entries = fs::read_dir(source).map_err(|error| io_error(source, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(source, error))?;
        let entry_relative = relative.join(entry.file_name());
        let metadata = entry
            .metadata()
            .map_err(|error| io_error(&entry.path(), error))?;
        if is_included(&filters, &entry_relative, metadata.is_dir()) {
            retained.insert(entry.file_name());
            copy_entry(
                &entry.path(),
                &destination.join(entry.file_name()),
                &entry_relative,
                options,
                program,
                &filters,
                context,
            )?;
        }
    }
    if options.delete {
        let destination_entries =
            fs::read_dir(destination).map_err(|error| io_error(destination, error))?;
        for entry in destination_entries {
            let entry = entry.map_err(|error| io_error(destination, error))?;
            let entry_relative = relative.join(entry.file_name());
            let metadata = entry
                .metadata()
                .map_err(|error| io_error(&entry.path(), error))?;
            let protected = !options.delete_excluded
                && !is_included(&filters, &entry_relative, metadata.is_dir());
            if !protected && !retained.contains(&entry.file_name()) {
                remove_entry(&entry.path())?;
            }
        }
    }
    if options.preserve_permissions {
        let metadata = fs::metadata(source).map_err(|error| io_error(source, error))?;
        fs::set_permissions(destination, metadata.permissions())
            .map_err(|error| io_error(destination, error))?;
    }
    if options.preserve_acls {
        copy_acl(source, destination)?;
    }
    Ok(())
}

fn copy_entry(
    source: &Path,
    destination: &Path,
    relative: &Path,
    options: &Options,
    program: &FilterProgram,
    active_rules: &[ActiveRule],
    context: &mut CopyContext,
) -> Result<(), ReferenceError> {
    let metadata = fs::symlink_metadata(source).map_err(|error| io_error(source, error))?;
    if metadata.file_type().is_symlink() {
        if options.copy_link_referents {
            let referent = fs::metadata(source).map_err(|error| io_error(source, error))?;
            copy_referent(
                source,
                destination,
                relative,
                referent,
                options,
                program,
                active_rules,
                context,
            )
        } else if options.preserve_symlinks
            && (!options.safe_links || safe_link_target(source, relative)?)
        {
            copy_symlink(source, destination)
        } else {
            Ok(())
        }
    } else if metadata.is_dir() {
        sync_directory(
            source,
            destination,
            relative,
            options,
            program,
            active_rules,
            context,
        )
    } else {
        copy_referent(
            source,
            destination,
            relative,
            metadata,
            options,
            program,
            active_rules,
            context,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_referent(
    source: &Path,
    destination: &Path,
    relative: &Path,
    metadata: fs::Metadata,
    options: &Options,
    program: &FilterProgram,
    active_rules: &[ActiveRule],
    context: &mut CopyContext,
) -> Result<(), ReferenceError> {
    if metadata.is_dir() {
        sync_directory(
            source,
            destination,
            relative,
            options,
            program,
            active_rules,
            context,
        )
    } else if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            create_directory(parent)?;
        }
        let identity = options
            .preserve_hard_links
            .then(|| Handle::from_path(source))
            .transpose()
            .map_err(|error| io_error(source, error))?;
        if let Some(existing) = identity.as_ref().and_then(|identity| {
            context
                .hard_links
                .iter()
                .find_map(|(known, destination)| (known == identity).then_some(destination))
        }) {
            if fs::symlink_metadata(destination).is_ok() {
                remove_entry(destination)?;
            }
            fs::hard_link(existing, destination).map_err(|error| io_error(destination, error))?;
            return Ok(());
        }
        if fs::symlink_metadata(destination).is_ok() {
            remove_entry(destination)?;
        }
        copy_file_data(source, destination)?;
        if let Some(identity) = identity {
            context
                .hard_links
                .push((identity, destination.to_path_buf()));
        }
        if options.preserve_xattrs {
            copy_extended_attributes(source, destination)?;
        }
        if options.sparse {
            preserve_sparse_allocation(destination)?;
        }
        if options.preserve_times {
            prepare_timestamp_write(destination, options.preserve_permissions)?;
            let modified = metadata
                .modified()
                .map_err(|error| io_error(source, error))?;
            let modified = modified
                .duration_since(UNIX_EPOCH)
                .map(|duration| UNIX_EPOCH + Duration::from_secs(duration.as_secs()))
                .unwrap_or(modified);
            let times = FileTimes::new().set_modified(modified);
            fs::OpenOptions::new()
                .write(true)
                .open(destination)
                .map_err(|error| io_error(destination, error))?
                .set_times(times)
                .map_err(|error| io_error(destination, error))?;
        }
        if options.preserve_permissions {
            fs::set_permissions(destination, metadata.permissions())
                .map_err(|error| io_error(destination, error))?;
        }
        if options.preserve_acls {
            copy_acl(source, destination)?;
        }
        Ok(())
    } else {
        Err(ReferenceError::Io {
            path: source.to_path_buf(),
            source: io::Error::new(io::ErrorKind::Unsupported, "unsupported entry kind"),
        })
    }
}

fn copy_file_data(source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    let mut input = fs::File::open(source).map_err(|error| io_error(source, error))?;
    let mut output = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(destination)
        .map_err(|error| io_error(destination, error))?;
    io::copy(&mut input, &mut output).map_err(|error| io_error(destination, error))?;
    output.flush().map_err(|error| io_error(destination, error))
}

#[cfg(windows)]
fn copy_acl(source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    const SCRIPT: &str = "$ErrorActionPreference='Stop'; $acl=Get-Acl -LiteralPath $env:HASEBENCH_ACL_SOURCE; Set-Acl -LiteralPath $env:HASEBENCH_ACL_DESTINATION -AclObject $acl";
    let output = Command::new("pwsh.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            SCRIPT,
        ])
        .env("HASEBENCH_ACL_SOURCE", source)
        .env("HASEBENCH_ACL_DESTINATION", destination)
        .output()
        .map_err(|error| capability_io_error("ACL preservation", destination, error))?;
    require_command_success("ACL preservation", destination, output)
}

#[cfg(not(windows))]
fn copy_acl(_source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    Err(unavailable_adapter(
        "ACL preservation",
        destination,
        "Unix ACL adapter is unavailable",
    ))
}

#[cfg(windows)]
fn copy_extended_attributes(source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    const SCRIPT: &str = "$ErrorActionPreference='Stop'; Get-Item -LiteralPath $env:HASEBENCH_STREAM_SOURCE -Stream * | Select-Object -ExpandProperty Stream";
    let output = Command::new("pwsh.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            SCRIPT,
        ])
        .env("HASEBENCH_STREAM_SOURCE", source)
        .output()
        .map_err(|error| capability_io_error("named-stream enumeration", source, error))?;
    if !output.status.success() {
        return require_command_success("named-stream enumeration", source, output);
    }
    for stream in String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|stream| !stream.is_empty() && *stream != ":$DATA")
    {
        copy_file_data(
            &named_stream_path(source, stream),
            &named_stream_path(destination, stream),
        )?;
    }
    Ok(())
}

#[cfg(windows)]
fn named_stream_path(path: &Path, stream: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(":");
    value.push(stream);
    PathBuf::from(value)
}

#[cfg(not(windows))]
fn copy_extended_attributes(_source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    Err(unavailable_adapter(
        "extended-attribute preservation",
        destination,
        "Unix xattr adapter is unavailable",
    ))
}

#[cfg(windows)]
fn preserve_sparse_allocation(path: &Path) -> Result<(), ReferenceError> {
    run_fsutil(path, "setflag", &[])?;
    let mut input = fs::File::open(path).map_err(|error| io_error(path, error))?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut offset = 0_u64;
    let mut zero_start = None;
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|error| io_error(path, error))?;
        if count == 0 {
            break;
        }
        if buffer[..count].iter().all(|byte| *byte == 0) {
            zero_start.get_or_insert(offset);
        } else if let Some(start) = zero_start.take() {
            set_sparse_range(path, start, offset - start)?;
        }
        offset += count as u64;
    }
    if let Some(start) = zero_start {
        set_sparse_range(path, start, offset - start)?;
    }
    Ok(())
}

#[cfg(windows)]
fn set_sparse_range(path: &Path, offset: u64, length: u64) -> Result<(), ReferenceError> {
    if length == 0 {
        return Ok(());
    }
    let offset = offset.to_string();
    let length = length.to_string();
    run_fsutil(path, "setrange", &[&offset, &length])
}

#[cfg(windows)]
fn run_fsutil(path: &Path, operation: &str, arguments: &[&str]) -> Result<(), ReferenceError> {
    let mut command = Command::new("fsutil.exe");
    command
        .args(["sparse", operation])
        .arg(path)
        .args(arguments);
    let output = command
        .output()
        .map_err(|error| capability_io_error("sparse-file preservation", path, error))?;
    require_command_success("sparse-file preservation", path, output)
}

#[cfg(not(windows))]
fn preserve_sparse_allocation(path: &Path) -> Result<(), ReferenceError> {
    Err(unavailable_adapter(
        "sparse-file preservation",
        path,
        "Unix sparse adapter is unavailable",
    ))
}

#[cfg(windows)]
fn require_command_success(
    feature: &'static str,
    path: &Path,
    output: std::process::Output,
) -> Result<(), ReferenceError> {
    if output.status.success() {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let diagnostic = if stdout.trim().is_empty() {
        stderr.trim().to_owned()
    } else {
        stdout.trim().to_owned()
    };
    Err(unavailable_adapter(feature, path, &diagnostic))
}

fn capability_io_error(feature: &'static str, path: &Path, error: io::Error) -> ReferenceError {
    ReferenceError::UnsupportedCapability {
        feature,
        path: path.to_path_buf(),
        source: error,
    }
}

fn unavailable_adapter(feature: &'static str, path: &Path, reason: &str) -> ReferenceError {
    capability_io_error(
        feature,
        path,
        io::Error::new(io::ErrorKind::Unsupported, reason.to_owned()),
    )
}

fn safe_link_target(source: &Path, relative: &Path) -> Result<bool, ReferenceError> {
    let target = fs::read_link(source).map_err(|error| io_error(source, error))?;
    Ok(link_target_is_safe(relative, &target))
}

fn link_target_is_safe(relative: &Path, target: &Path) -> bool {
    use std::path::Component;

    if target.is_absolute() {
        return false;
    }
    let mut depth = relative
        .parent()
        .map_or(0, |parent| parent.components().count());
    for component in target.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir if depth > 0 => depth -= 1,
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

#[cfg(windows)]
#[allow(clippy::permissions_set_readonly_false)]
fn prepare_timestamp_write(
    destination: &Path,
    permissions_will_be_restored: bool,
) -> Result<(), ReferenceError> {
    let mut permissions = fs::metadata(destination)
        .map_err(|error| io_error(destination, error))?
        .permissions();
    if permissions_will_be_restored && permissions.readonly() {
        permissions.set_readonly(false);
        fs::set_permissions(destination, permissions)
            .map_err(|error| io_error(destination, error))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn prepare_timestamp_write(
    _destination: &Path,
    _permissions_will_be_restored: bool,
) -> Result<(), ReferenceError> {
    Ok(())
}

fn expand_filters(options: &Options) -> Result<FilterProgram, ReferenceError> {
    let mut program = FilterProgram::default();
    for directive in &options.filters {
        match directive {
            FilterDirective::Rule(rule) => program.global_rules.push(ActiveRule {
                rule: rule.clone(),
                base: PathBuf::new(),
            }),
            FilterDirective::File(file) => {
                if file.kind == FilterFileKind::DirMerge {
                    program.dir_merge_files.push(file.path.clone());
                } else {
                    program.global_rules.extend(read_filter_file(
                        &file.path,
                        file.kind,
                        Path::new(""),
                    )?);
                }
            }
        }
    }
    Ok(program)
}

fn read_filter_file(
    path: &Path,
    kind: FilterFileKind,
    base: &Path,
) -> Result<Vec<ActiveRule>, ReferenceError> {
    let contents = fs::read_to_string(path).map_err(|error| io_error(path, error))?;
    let mut rules = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let rule = match kind {
            FilterFileKind::Include => FilterRule {
                include: true,
                pattern: line.to_owned(),
            },
            FilterFileKind::Exclude => FilterRule {
                include: false,
                pattern: line.to_owned(),
            },
            FilterFileKind::Merge | FilterFileKind::DirMerge => parse_merge_rule(line)?,
        };
        rules.push(ActiveRule {
            rule,
            base: base.to_path_buf(),
        });
    }
    Ok(rules)
}

fn parse_merge_rule(line: &str) -> Result<FilterRule, ReferenceError> {
    let (include, pattern) = if let Some(pattern) = line.strip_prefix("+ ") {
        (true, pattern)
    } else if let Some(pattern) = line.strip_prefix("- ") {
        (false, pattern)
    } else {
        return Err(ReferenceError::InvalidFilter(line.to_owned()));
    };
    if pattern.is_empty() {
        return Err(ReferenceError::InvalidFilter(line.to_owned()));
    }
    Ok(FilterRule {
        include,
        pattern: pattern.to_owned(),
    })
}

fn is_included(filters: &[ActiveRule], relative: &Path, is_directory: bool) -> bool {
    let path = relative.to_string_lossy().replace('\\', "/");
    filters
        .iter()
        .find_map(|active| {
            let base = active.base.to_string_lossy().replace('\\', "/");
            let local = if base.is_empty() {
                path.as_str()
            } else {
                path.strip_prefix(&format!("{base}/"))?
            };
            rule_matches(&active.rule, local, is_directory).then_some(active.rule.include)
        })
        .unwrap_or(true)
}

fn rule_matches(rule: &FilterRule, path: &str, is_directory: bool) -> bool {
    let directory_only = rule.pattern.ends_with('/');
    if directory_only && !is_directory {
        return false;
    }
    let mut pattern = rule.pattern.trim_end_matches('/');
    let anchored = pattern.starts_with('/');
    pattern = pattern.trim_start_matches('/');
    if let Some(prefix) = pattern.strip_suffix("/***") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    let candidate = if anchored || pattern.contains('/') {
        path
    } else {
        path.rsplit('/').next().unwrap_or(path)
    };
    wildcard_matches(pattern.as_bytes(), candidate.as_bytes())
}

fn wildcard_matches(pattern: &[u8], value: &[u8]) -> bool {
    fn matches(pattern: &[u8], value: &[u8]) -> bool {
        match pattern {
            [] => value.is_empty(),
            [b'*', b'*', rest @ ..] => {
                matches(rest, value) || (!value.is_empty() && matches(pattern, &value[1..]))
            }
            [b'*', rest @ ..] => {
                matches(rest, value)
                    || (!value.is_empty() && value[0] != b'/' && matches(pattern, &value[1..]))
            }
            [b'?', rest @ ..] => {
                !value.is_empty() && value[0] != b'/' && matches(rest, &value[1..])
            }
            [first, rest @ ..] => value.first() == Some(first) && matches(rest, &value[1..]),
        }
    }
    matches(pattern, value)
}

fn create_directory(path: &Path) -> Result<(), ReferenceError> {
    fs::create_dir_all(path).map_err(|error| io_error(path, error))
}

fn remove_entry(path: &Path) -> Result<(), ReferenceError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path).map_err(|error| io_error(path, error))
    } else {
        fs::remove_file(path).map_err(|error| io_error(path, error))
    }
}

#[cfg(unix)]
fn copy_symlink(source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    use std::os::unix::fs::symlink;

    if fs::symlink_metadata(destination).is_ok() {
        remove_entry(destination)?;
    }
    let target = fs::read_link(source).map_err(|error| io_error(source, error))?;
    symlink(target, destination).map_err(|error| io_error(destination, error))
}

#[cfg(windows)]
fn copy_symlink(source: &Path, destination: &Path) -> Result<(), ReferenceError> {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    if fs::symlink_metadata(destination).is_ok() {
        remove_entry(destination)?;
    }
    let target = fs::read_link(source).map_err(|error| io_error(source, error))?;
    let referent = source
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&target);
    let result = if referent.is_dir() {
        symlink_dir(target, destination)
    } else {
        symlink_file(target, destination)
    };
    result.map_err(|error| {
        if error.kind() == io::ErrorKind::PermissionDenied {
            ReferenceError::UnsupportedCapability {
                feature: "symbolic-link creation",
                path: destination.to_path_buf(),
                source: error,
            }
        } else {
            io_error(destination, error)
        }
    })
}

fn io_error(path: &Path, source: io::Error) -> ReferenceError {
    ReferenceError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::link_target_is_safe;
    use std::path::Path;

    #[test]
    fn safe_link_targets_remain_inside_the_transferred_tree() {
        assert!(link_target_is_safe(
            Path::new("nested/link"),
            Path::new("../inside")
        ));
        assert!(link_target_is_safe(
            Path::new("nested/link"),
            Path::new("child")
        ));
        assert!(!link_target_is_safe(
            Path::new("nested/link"),
            Path::new("../../outside")
        ));
        assert!(!link_target_is_safe(
            Path::new("link"),
            Path::new("../outside")
        ));
        assert!(!link_target_is_safe(
            Path::new("link"),
            Path::new("/absolute")
        ));
    }
}
