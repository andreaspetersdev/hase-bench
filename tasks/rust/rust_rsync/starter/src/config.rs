use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRule {
    pub include: bool,
    pub pattern: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterFileKind {
    Include,
    Exclude,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterFile {
    pub kind: FilterFileKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterDirective {
    Rule(FilterRule),
    File(FilterFile),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonModule {
    pub name: String,
    pub root: PathBuf,
    pub read_only: bool,
}
