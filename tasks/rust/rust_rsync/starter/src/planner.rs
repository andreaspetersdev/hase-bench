use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedOperation {
    CreateDirectory(PathBuf),
    TransferFile(PathBuf),
    CreateSymlink(PathBuf),
    Remove(PathBuf),
    ApplyMetadata(PathBuf),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransferPlan {
    pub operations: Vec<PlannedOperation>,
}
