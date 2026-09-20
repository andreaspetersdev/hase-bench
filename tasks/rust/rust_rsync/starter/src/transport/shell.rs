use std::ffi::OsString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: OsString,
    pub arguments: Vec<OsString>,
}
