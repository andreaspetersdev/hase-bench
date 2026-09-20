use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMetadata {
    pub relative_path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub link_target: Option<OsString>,
}

pub trait FileSystem {
    type Error;

    fn list(&self, root: &Path) -> Result<Vec<EntryMetadata>, Self::Error>;
}
