use std::path::PathBuf;

use crate::fs::CapabilityOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: PathBuf,
    pub kind: String,
    pub size: u64,
    pub digest: Option<String>,
    pub capabilities: Vec<CapabilityOutcome>,
}
