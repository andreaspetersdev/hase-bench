use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: PathBuf,
    pub kind: String,
    pub size: u64,
    pub digest: Option<String>,
    pub capabilities: Vec<String>,
}
