use std::fmt;

use crate::cli::Invocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    NotImplemented,
}

impl RunError {
    pub fn exit_code(&self) -> u8 {
        1
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("transfer engine is not implemented")
    }
}

impl std::error::Error for RunError {}

pub fn run(_invocation: Invocation) -> Result<(), RunError> {
    Err(RunError::NotImplemented)
}
