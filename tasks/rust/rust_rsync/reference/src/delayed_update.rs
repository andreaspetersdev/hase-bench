use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TRANSACTION_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedUpdate {
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DelayedUpdateFault {
    pub interrupt_after_staged: Option<usize>,
    pub fail_before_commit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedUpdateResult {
    pub committed_files: usize,
}

#[derive(Debug)]
pub enum DelayedUpdateError {
    InvalidRoot(PathBuf),
    InvalidRelativePath(PathBuf),
    DuplicateDestination(PathBuf),
    Interrupted {
        staged_files: usize,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Commit {
        path: PathBuf,
        source: io::Error,
        rollback_error: Option<String>,
    },
}

impl fmt::Display for DelayedUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(path) => write!(
                formatter,
                "delayed-update root must be an existing real directory: {}",
                path.display()
            ),
            Self::InvalidRelativePath(path) => write!(
                formatter,
                "delayed-update destination must be a confined relative file path: {}",
                path.display()
            ),
            Self::DuplicateDestination(path) => {
                write!(
                    formatter,
                    "duplicate delayed-update destination: {}",
                    path.display()
                )
            }
            Self::Interrupted { staged_files } => {
                write!(
                    formatter,
                    "transfer interrupted after staging {staged_files} files"
                )
            }
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Commit {
                path,
                source,
                rollback_error,
            } => {
                write!(formatter, "failed to commit {}: {source}", path.display())?;
                if let Some(rollback_error) = rollback_error {
                    write!(formatter, "; rollback also failed: {rollback_error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DelayedUpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::Commit { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct CommittedFile {
    destination: PathBuf,
    backup: Option<PathBuf>,
    installed: bool,
}

pub fn apply_delayed_updates(
    root: &Path,
    updates: &[DelayedUpdate],
    fault: DelayedUpdateFault,
) -> Result<DelayedUpdateResult, DelayedUpdateError> {
    validate_root(root)?;
    validate_updates(root, updates)?;
    let transaction = create_transaction_directory(root)?;
    let staged_root = transaction.join("new");
    let backup_root = transaction.join("old");

    for (index, update) in updates.iter().enumerate() {
        if fault.interrupt_after_staged == Some(index) {
            cleanup_transaction(&transaction)?;
            return Err(DelayedUpdateError::Interrupted {
                staged_files: index,
            });
        }
        let staged = staged_root.join(&update.relative_path);
        if let Err(error) = write_staged_file(&staged, &update.contents) {
            let _ = fs::remove_dir_all(&transaction);
            return Err(error);
        }
    }
    if fault.interrupt_after_staged == Some(updates.len()) {
        cleanup_transaction(&transaction)?;
        return Err(DelayedUpdateError::Interrupted {
            staged_files: updates.len(),
        });
    }

    let mut committed = Vec::with_capacity(updates.len());
    let mut created_directories = Vec::new();
    for (index, update) in updates.iter().enumerate() {
        let destination = root.join(&update.relative_path);
        let staged = staged_root.join(&update.relative_path);
        let backup = backup_root.join(index.to_string());
        let result = commit_one(
            index,
            &destination,
            &staged,
            &backup,
            fault,
            &mut committed,
            &mut created_directories,
            root,
        );
        if let Err(source) = result {
            let rollback_error = rollback(&mut committed, &created_directories);
            let _ = fs::remove_dir_all(&transaction);
            return Err(DelayedUpdateError::Commit {
                path: destination,
                source,
                rollback_error,
            });
        }
    }

    cleanup_transaction(&transaction)?;
    Ok(DelayedUpdateResult {
        committed_files: updates.len(),
    })
}

#[allow(clippy::too_many_arguments)]
fn commit_one(
    index: usize,
    destination: &Path,
    staged: &Path,
    backup: &Path,
    fault: DelayedUpdateFault,
    committed: &mut Vec<CommittedFile>,
    created_directories: &mut Vec<PathBuf>,
    root: &Path,
) -> io::Result<()> {
    if fault.fail_before_commit == Some(index) {
        return Err(io::Error::other("injected delayed-update commit failure"));
    }
    create_parent_directories(root, destination, created_directories)?;
    let existing = match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => true,
        Ok(_) => {
            return Err(io::Error::other(
                "destination is not a replaceable regular file",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    if existing {
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(destination, backup)?;
    }
    committed.push(CommittedFile {
        destination: destination.to_path_buf(),
        backup: existing.then(|| backup.to_path_buf()),
        installed: false,
    });
    fs::rename(staged, destination)?;
    committed.last_mut().expect("just pushed").installed = true;
    Ok(())
}

fn validate_root(root: &Path) -> Result<(), DelayedUpdateError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        _ => Err(DelayedUpdateError::InvalidRoot(root.to_path_buf())),
    }
}

fn validate_updates(root: &Path, updates: &[DelayedUpdate]) -> Result<(), DelayedUpdateError> {
    let mut destinations = BTreeSet::new();
    for update in updates {
        validate_relative_path(root, &update.relative_path)?;
        if !destinations.insert(update.relative_path.clone()) {
            return Err(DelayedUpdateError::DuplicateDestination(
                update.relative_path.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_relative_path(root: &Path, relative: &Path) -> Result<(), DelayedUpdateError> {
    if relative.as_os_str().is_empty() || relative.is_absolute() {
        return Err(DelayedUpdateError::InvalidRelativePath(
            relative.to_path_buf(),
        ));
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => {
                current.push(name);
                if fs::symlink_metadata(&current)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return Err(DelayedUpdateError::InvalidRelativePath(
                        relative.to_path_buf(),
                    ));
                }
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(DelayedUpdateError::InvalidRelativePath(
                    relative.to_path_buf(),
                ));
            }
        }
    }
    Ok(())
}

fn create_transaction_directory(root: &Path) -> Result<PathBuf, DelayedUpdateError> {
    for _ in 0..100 {
        let id = TRANSACTION_ID.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!(".rust-rsync-delay-{}-{id}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io_error(&path, error)),
        }
    }
    let path = root.join(".rust-rsync-delay");
    Err(io_error(
        &path,
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique delayed-update directory",
        ),
    ))
}

fn write_staged_file(path: &Path, contents: &[u8]) -> Result<(), DelayedUpdateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    }
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error(path, error))?;
    output
        .write_all(contents)
        .and_then(|()| output.sync_all())
        .map_err(|error| io_error(path, error))
}

fn create_parent_directories(
    root: &Path,
    destination: &Path,
    created: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let relative_parent = destination
        .parent()
        .and_then(|parent| parent.strip_prefix(root).ok())
        .unwrap_or_else(|| Path::new(""));
    let mut current = root.to_path_buf();
    for component in relative_parent.components() {
        current.push(component);
        match fs::create_dir(&current) {
            Ok(()) => created.push(current.clone()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&current)?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(io::Error::other(
                        "destination parent is not a real directory",
                    ));
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn rollback(committed: &mut [CommittedFile], created_directories: &[PathBuf]) -> Option<String> {
    let mut failures = Vec::new();
    for entry in committed.iter().rev() {
        if entry.installed
            && let Err(error) = fs::remove_file(&entry.destination)
        {
            failures.push(format!("remove {}: {error}", entry.destination.display()));
        }
        if let Some(backup) = &entry.backup
            && let Err(error) = fs::rename(backup, &entry.destination)
        {
            failures.push(format!("restore {}: {error}", entry.destination.display()));
        }
    }
    for directory in created_directories.iter().rev() {
        match fs::remove_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => failures.push(format!("remove {}: {error}", directory.display())),
        }
    }
    (!failures.is_empty()).then(|| failures.join("; "))
}

fn cleanup_transaction(path: &Path) -> Result<(), DelayedUpdateError> {
    fs::remove_dir_all(path).map_err(|error| io_error(path, error))
}

fn io_error(path: &Path, source: io::Error) -> DelayedUpdateError {
    DelayedUpdateError::Io {
        path: path.to_path_buf(),
        source,
    }
}
