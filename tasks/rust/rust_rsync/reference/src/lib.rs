#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, FileTimes};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use rust_rsync::config::{FilterDirective, FilterFileKind, FilterRule};
use rust_rsync::{Endpoint, Invocation, Options, PathSpec};

#[derive(Debug)]
pub enum ReferenceError {
    UnsupportedMode,
    InvalidDestination,
    InvalidFilter(String),
    MissingFileName(PathBuf),
    Io { path: PathBuf, source: io::Error },
}

impl ReferenceError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnsupportedMode
            | Self::InvalidDestination
            | Self::InvalidFilter(_)
            | Self::MissingFileName(_) => 2,
            Self::Io { .. } => 23,
        }
    }
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMode => {
                formatter.write_str("reference phase supports local endpoints only")
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
            Self::Io { source, .. } => Some(source),
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
    let filters = expand_filters(&options)?;
    if options.dry_run {
        return Ok(());
    }
    if sources.len() > 1 && !destination.path.is_dir() {
        return Err(ReferenceError::InvalidDestination);
    }
    for source in &sources {
        transfer_source(source, &destination.path, &options, &filters)?;
    }
    Ok(())
}

fn transfer_source(
    source: &PathSpec,
    destination: &Path,
    options: &Options,
    filters: &[FilterRule],
) -> Result<(), ReferenceError> {
    let metadata =
        fs::symlink_metadata(&source.path).map_err(|error| io_error(&source.path, error))?;
    if metadata.is_dir() && source.copy_contents {
        create_directory(destination)?;
        sync_directory(&source.path, destination, Path::new(""), options, filters)
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
        copy_entry(&source.path, &target, Path::new(name), options, filters)
    }
}

fn sync_directory(
    source: &Path,
    destination: &Path,
    relative: &Path,
    options: &Options,
    filters: &[FilterRule],
) -> Result<(), ReferenceError> {
    create_directory(destination)?;
    let mut retained = BTreeSet::new();
    let entries = fs::read_dir(source).map_err(|error| io_error(source, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(source, error))?;
        let entry_relative = relative.join(entry.file_name());
        let metadata = entry
            .metadata()
            .map_err(|error| io_error(&entry.path(), error))?;
        if is_included(filters, &entry_relative, metadata.is_dir()) {
            retained.insert(entry.file_name());
            copy_entry(
                &entry.path(),
                &destination.join(entry.file_name()),
                &entry_relative,
                options,
                filters,
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
                && !is_included(filters, &entry_relative, metadata.is_dir());
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
    Ok(())
}

fn copy_entry(
    source: &Path,
    destination: &Path,
    relative: &Path,
    options: &Options,
    filters: &[FilterRule],
) -> Result<(), ReferenceError> {
    let metadata = fs::symlink_metadata(source).map_err(|error| io_error(source, error))?;
    if metadata.file_type().is_symlink() {
        copy_symlink(source, destination)
    } else if metadata.is_dir() {
        sync_directory(source, destination, relative, options, filters)
    } else if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            create_directory(parent)?;
        }
        fs::copy(source, destination).map_err(|error| io_error(destination, error))?;
        if options.preserve_permissions {
            fs::set_permissions(destination, metadata.permissions())
                .map_err(|error| io_error(destination, error))?;
        }
        if options.preserve_times {
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
        Ok(())
    } else {
        Err(ReferenceError::Io {
            path: source.to_path_buf(),
            source: io::Error::new(io::ErrorKind::Unsupported, "unsupported entry kind"),
        })
    }
}

fn expand_filters(options: &Options) -> Result<Vec<FilterRule>, ReferenceError> {
    let mut expanded = Vec::new();
    for directive in &options.filters {
        match directive {
            FilterDirective::Rule(rule) => expanded.push(rule.clone()),
            FilterDirective::File(file) => {
                let contents =
                    fs::read_to_string(&file.path).map_err(|error| io_error(&file.path, error))?;
                for line in contents.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let rule = match file.kind {
                        FilterFileKind::Include => FilterRule {
                            include: true,
                            pattern: line.to_owned(),
                        },
                        FilterFileKind::Exclude => FilterRule {
                            include: false,
                            pattern: line.to_owned(),
                        },
                        FilterFileKind::Merge => parse_merge_rule(line)?,
                    };
                    expanded.push(rule);
                }
            }
        }
    }
    Ok(expanded)
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

fn is_included(filters: &[FilterRule], relative: &Path, is_directory: bool) -> bool {
    let path = relative.to_string_lossy().replace('\\', "/");
    filters
        .iter()
        .find_map(|rule| rule_matches(rule, &path, is_directory).then_some(rule.include))
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
    result.map_err(|error| io_error(destination, error))
}

fn io_error(path: &Path, source: io::Error) -> ReferenceError {
    ReferenceError::Io {
        path: path.to_path_buf(),
        source,
    }
}
