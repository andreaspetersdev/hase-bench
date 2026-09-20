use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetadataFeature {
    ModificationTimes,
    Permissions,
    Ownership,
    Acls,
    ExtendedAttributes,
    Symlinks,
    HardLinks,
    SparseFiles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityStatus {
    Supported,
    ProbeRequired,
    HostUnsupported { reason: String },
    AdapterUnavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOutcome {
    pub feature: MetadataFeature,
    pub status: CapabilityStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityReport {
    pub outcomes: Vec<CapabilityOutcome>,
}

impl CapabilityReport {
    pub fn status(&self, feature: MetadataFeature) -> Option<&CapabilityStatus> {
        self.outcomes
            .iter()
            .find(|outcome| outcome.feature == feature)
            .map(|outcome| &outcome.status)
    }
}

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

    fn capabilities(&self) -> CapabilityReport;
    fn list(&self, root: &Path) -> Result<Vec<EntryMetadata>, Self::Error>;
}
