use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRule {
    pub include: bool,
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonModule {
    pub name: String,
    pub root: PathBuf,
    pub read_only: bool,
}
