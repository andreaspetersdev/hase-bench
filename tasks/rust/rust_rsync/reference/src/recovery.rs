use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoveryPolicy {
    pub retain_partial: bool,
    pub partial_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryResult {
    pub reused_bytes: u64,
    pub written_bytes: u64,
}

#[derive(Debug)]
pub enum RecoveryError {
    InvalidPartialDirectory(PathBuf),
    MissingFileName(PathBuf),
    Interrupted {
        completed_bytes: u64,
        retained_path: Option<PathBuf>,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPartialDirectory(path) => write!(
                formatter,
                "partial directory must be a confined relative path without symlinks: {}",
                path.display()
            ),
            Self::MissingFileName(path) => {
                write!(
                    formatter,
                    "destination has no file name: {}",
                    path.display()
                )
            }
            Self::Interrupted {
                completed_bytes,
                retained_path,
            } => match retained_path {
                Some(path) => write!(
                    formatter,
                    "transfer interrupted after {completed_bytes} bytes; partial retained at {}",
                    path.display()
                ),
                None => write!(
                    formatter,
                    "transfer interrupted after {completed_bytes} bytes; partial removed"
                ),
            },
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for RecoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn write_with_recovery(
    destination: &Path,
    contents: &[u8],
    policy: &RecoveryPolicy,
    interrupt_after: Option<u64>,
) -> Result<RecoveryResult, RecoveryError> {
    let partial = partial_path(destination, policy)?;
    if let Some(parent) = partial.parent() {
        fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
    }

    let reused_bytes = if policy.retain_partial {
        reusable_prefix(&partial, contents)?
    } else {
        remove_file_if_present(&partial)?;
        0
    };
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&partial)
        .map_err(|error| io_error(&partial, error))?;
    output
        .seek(SeekFrom::Start(reused_bytes))
        .map_err(|error| io_error(&partial, error))?;

    let content_len = u64::try_from(contents.len()).expect("usize fits in u64 on supported hosts");
    let attempt_end = interrupt_after.unwrap_or(content_len).min(content_len);
    let write_end = attempt_end.max(reused_bytes).min(content_len);
    let start = usize::try_from(reused_bytes).expect("reused prefix came from this slice");
    let end = usize::try_from(write_end).expect("write end came from this slice");
    output
        .write_all(&contents[start..end])
        .map_err(|error| io_error(&partial, error))?;
    output
        .sync_all()
        .map_err(|error| io_error(&partial, error))?;

    if write_end < content_len {
        drop(output);
        let retained_path = if policy.retain_partial {
            Some(partial)
        } else {
            remove_file_if_present(&partial)?;
            None
        };
        return Err(RecoveryError::Interrupted {
            completed_bytes: write_end,
            retained_path,
        });
    }
    drop(output);

    if partial != destination {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
        }
        remove_file_if_present(destination)?;
        fs::rename(&partial, destination).map_err(|error| io_error(destination, error))?;
    }
    Ok(RecoveryResult {
        reused_bytes,
        written_bytes: content_len - reused_bytes,
    })
}

fn partial_path(destination: &Path, policy: &RecoveryPolicy) -> Result<PathBuf, RecoveryError> {
    if policy.retain_partial && policy.partial_dir.is_none() {
        return Ok(destination.to_path_buf());
    }
    let name = destination
        .file_name()
        .ok_or_else(|| RecoveryError::MissingFileName(destination.to_path_buf()))?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    if let Some(directory) = &policy.partial_dir {
        validate_partial_directory(parent, directory)?;
        Ok(parent.join(directory).join(name))
    } else {
        let mut temporary = name.to_os_string();
        temporary.push(".rust-rsync-partial");
        Ok(parent.join(temporary))
    }
}

fn validate_partial_directory(root: &Path, directory: &Path) -> Result<(), RecoveryError> {
    if directory.as_os_str().is_empty() || directory.is_absolute() {
        return Err(RecoveryError::InvalidPartialDirectory(
            directory.to_path_buf(),
        ));
    }
    let mut current = root.to_path_buf();
    for component in directory.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => {
                current.push(name);
                if fs::symlink_metadata(&current)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return Err(RecoveryError::InvalidPartialDirectory(
                        directory.to_path_buf(),
                    ));
                }
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(RecoveryError::InvalidPartialDirectory(
                    directory.to_path_buf(),
                ));
            }
        }
    }
    Ok(())
}

fn reusable_prefix(path: &Path, contents: &[u8]) -> Result<u64, RecoveryError> {
    let mut input = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(io_error(path, error)),
    };
    let length = input
        .metadata()
        .map_err(|error| io_error(path, error))?
        .len();
    let Ok(length_usize) = usize::try_from(length) else {
        input.set_len(0).map_err(|error| io_error(path, error))?;
        return Ok(0);
    };
    if length_usize > contents.len() {
        input.set_len(0).map_err(|error| io_error(path, error))?;
        return Ok(0);
    }
    let mut compared = 0_usize;
    let mut buffer = [0_u8; 64 * 1024];
    while compared < length_usize {
        let count = (length_usize - compared).min(buffer.len());
        input
            .read_exact(&mut buffer[..count])
            .map_err(|error| io_error(path, error))?;
        if buffer[..count] != contents[compared..compared + count] {
            input.set_len(0).map_err(|error| io_error(path, error))?;
            return Ok(0);
        }
        compared += count;
    }
    Ok(length)
}

fn remove_file_if_present(path: &Path) -> Result<(), RecoveryError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(path, error)),
    }
}

fn io_error(path: &Path, source: io::Error) -> RecoveryError {
    RecoveryError::Io {
        path: path.to_path_buf(),
        source,
    }
}
