use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CounterStore {
    values: BTreeMap<String, i64>,
}

impl CounterStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, value: i64) -> Option<i64> {
        self.values.insert(key.into(), value)
    }

    pub fn get(&self, key: &str) -> Option<i64> {
        self.values.get(key).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOperationError {
    MissingEquals,
    EmptyKey,
    InvalidDelta { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    MissingKey { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    Overflow {
        key: String,
        current: i64,
        delta: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    Parse(ParseOperationError),
    Lookup(LookupError),
    Update(UpdateError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineError {
    pub operation_index: usize,
    pub error: OperationError,
}

impl fmt::Display for ParseOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "operation syntax is invalid")
    }
}

impl Error for ParseOperationError {}

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "counter lookup failed")
    }
}

impl Error for LookupError {}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "counter update failed")
    }
}

impl Error for UpdateError {}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "operation failed")
    }
}

impl Error for OperationError {}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "operation {} failed", self.operation_index)
    }
}

impl Error for PipelineError {}

pub fn apply_operations(
    _store: &mut CounterStore,
    _operations: &[&str],
) -> Result<Vec<i64>, PipelineError> {
    Ok(Vec::new())
}
