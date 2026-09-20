#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Get { key: String },
    Put { key: String, value: Vec<u8> },
    Delete { key: String },
}

impl Operation {
    pub fn key(&self) -> &str {
        match self {
            Self::Get { key } | Self::Put { key, .. } | Self::Delete { key } => key,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddlewarePhase {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    Backend(String),
    Conflict(String),
    TransactionClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiddlewareError {
    Rejected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationFailure {
    Middleware {
        phase: MiddlewarePhase,
        source: MiddlewareError,
    },
    Storage(StorageError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    InvalidKey {
        operation: usize,
        key: String,
    },
    Begin(StorageError),
    Operation {
        operation: usize,
        source: OperationFailure,
    },
    Commit(StorageError),
    Rollback {
        original: Box<ServiceError>,
        rollback: StorageError,
    },
}
