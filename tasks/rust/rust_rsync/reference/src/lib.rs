#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rust_rsync::{Endpoint, Invocation, PathSpec};

#[derive(Debug)]
pub enum ReferenceError {
    UnsupportedMode,
    InvalidDestination,
    MissingFileName(PathBuf),
    Io { path: PathBuf, source: io::Error },
}

impl ReferenceError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnsupportedMode | Self::InvalidDestination | Self::MissingFileName(_) => 2,
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
    let Endpoint::Local(destination) = invocation.destination else {
        return Err(ReferenceError::UnsupportedMode);
    };
    let sources = invocation
        .sources
        .into_iter()
        .map(|source| match source {
            Endpoint::Local(path) => Ok(path),
            _ => Err(ReferenceError::UnsupportedMode),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if invocation.options.dry_run {
        return Ok(());
    }
    if sources.len() > 1 && !destination.path.is_dir() {
        return Err(ReferenceError::InvalidDestination);
    }
    for source in &sources {
        transfer_source(source, &destination.path, invocation.options.delete)?;
    }
    Ok(())
}

fn transfer_source(
    source: &PathSpec,
    destination: &Path,
    delete: bool,
) -> Result<(), ReferenceError> {
    let metadata =
        fs::symlink_metadata(&source.path).map_err(|error| io_error(&source.path, error))?;
    if metadata.is_dir() && source.copy_contents {
        create_directory(destination)?;
        sync_directory(&source.path, destination, delete)
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
        copy_entry(&source.path, &target, delete)
    }
}

fn sync_directory(source: &Path, destination: &Path, delete: bool) -> Result<(), ReferenceError> {
    create_directory(destination)?;
    let mut retained = BTreeSet::new();
    let entries = fs::read_dir(source).map_err(|error| io_error(source, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(source, error))?;
        retained.insert(entry.file_name());
        copy_entry(&entry.path(), &destination.join(entry.file_name()), delete)?;
    }
    if delete {
        let destination_entries =
            fs::read_dir(destination).map_err(|error| io_error(destination, error))?;
        for entry in destination_entries {
            let entry = entry.map_err(|error| io_error(destination, error))?;
            if !retained.contains(&entry.file_name()) {
                remove_entry(&entry.path())?;
            }
        }
    }
    Ok(())
}

fn copy_entry(source: &Path, destination: &Path, delete: bool) -> Result<(), ReferenceError> {
    let metadata = fs::symlink_metadata(source).map_err(|error| io_error(source, error))?;
    if metadata.file_type().is_symlink() {
        copy_symlink(source, destination)
    } else if metadata.is_dir() {
        sync_directory(source, destination, delete)
    } else if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            create_directory(parent)?;
        }
        fs::copy(source, destination).map_err(|error| io_error(destination, error))?;
        fs::set_permissions(destination, metadata.permissions())
            .map_err(|error| io_error(destination, error))?;
        Ok(())
    } else {
        Err(ReferenceError::Io {
            path: source.to_path_buf(),
            source: io::Error::new(io::ErrorKind::Unsupported, "unsupported entry kind"),
        })
    }
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
