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
        match self {
            Self::MissingEquals => write!(formatter, "operation is missing '='"),
            Self::EmptyKey => write!(formatter, "operation key is empty"),
            Self::InvalidDelta { value } => write!(formatter, "invalid delta '{value}'"),
        }
    }
}

impl Error for ParseOperationError {}

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey { key } => write!(formatter, "counter '{key}' does not exist"),
        }
    }
}

impl Error for LookupError {}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow {
                key,
                current,
                delta,
            } => write!(
                formatter,
                "updating counter '{key}' from {current} by {delta} would overflow"
            ),
        }
    }
}

impl Error for UpdateError {}

impl From<ParseOperationError> for OperationError {
    fn from(error: ParseOperationError) -> Self {
        Self::Parse(error)
    }
}

impl From<LookupError> for OperationError {
    fn from(error: LookupError) -> Self {
        Self::Lookup(error)
    }
}

impl From<UpdateError> for OperationError {
    fn from(error: UpdateError) -> Self {
        Self::Update(error)
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "cannot parse operation: {error}"),
            Self::Lookup(error) => write!(formatter, "cannot find counter: {error}"),
            Self::Update(error) => write!(formatter, "cannot apply update: {error}"),
        }
    }
}

impl Error for OperationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Lookup(error) => Some(error),
            Self::Update(error) => Some(error),
        }
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "operation {} failed: {}",
            self.operation_index, self.error
        )
    }
}

impl Error for PipelineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

pub fn apply_operations(
    store: &mut CounterStore,
    operations: &[&str],
) -> Result<Vec<i64>, PipelineError> {
    let mut results = Vec::with_capacity(operations.len());

    for (operation_index, text) in operations.iter().enumerate() {
        let result = apply_one(store, text).map_err(|error| PipelineError {
            operation_index,
            error,
        })?;
        results.push(result);
    }

    Ok(results)
}

fn apply_one(store: &mut CounterStore, text: &str) -> Result<i64, OperationError> {
    let (raw_key, raw_delta) = text
        .split_once('=')
        .ok_or(ParseOperationError::MissingEquals)?;
    let key = trim_horizontal(raw_key);
    if key.is_empty() {
        return Err(ParseOperationError::EmptyKey.into());
    }
    let delta_text = trim_horizontal(raw_delta);
    let delta = delta_text
        .parse::<i64>()
        .map_err(|_| ParseOperationError::InvalidDelta {
            value: delta_text.to_owned(),
        })?;
    let current = store.get(key).ok_or_else(|| LookupError::MissingKey {
        key: key.to_owned(),
    })?;
    let updated = current
        .checked_add(delta)
        .ok_or_else(|| UpdateError::Overflow {
            key: key.to_owned(),
            current,
            delta,
        })?;
    store.insert(key, updated);
    Ok(updated)
}

fn trim_horizontal(text: &str) -> &str {
    text.trim_matches(|character| matches!(character, ' ' | '\t'))
}
