use crate::model::{MiddlewareError, MiddlewarePhase, Operation, OperationFailure, ServiceError};
use crate::storage::{Storage, Transaction};

pub trait Middleware {
    fn handle(
        &mut self,
        phase: MiddlewarePhase,
        operation_index: usize,
        operation: &Operation,
    ) -> Result<(), MiddlewareError>;
}

pub struct DocumentService {
    storage: Box<dyn Storage>,
    middleware: Vec<Box<dyn Middleware>>,
}

impl DocumentService {
    pub fn new<S: Storage + 'static>(storage: S) -> Self {
        Self {
            storage: Box::new(storage),
            middleware: Vec::new(),
        }
    }

    pub fn add_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middleware.push(Box::new(middleware));
    }

    pub fn execute(
        &mut self,
        operations: Vec<Operation>,
    ) -> Result<Vec<Option<Vec<u8>>>, ServiceError> {
        validate(&operations)?;
        if operations.is_empty() {
            return Ok(Vec::new());
        }

        let Self {
            storage,
            middleware,
        } = self;
        let mut transaction = storage.begin().map_err(ServiceError::Begin)?;
        let mut results = Vec::with_capacity(operations.len());

        for (index, operation) in operations.iter().enumerate() {
            for hook in middleware.iter_mut() {
                if let Err(source) = hook.handle(MiddlewarePhase::Before, index, operation) {
                    let original = ServiceError::Operation {
                        operation: index,
                        source: OperationFailure::Middleware {
                            phase: MiddlewarePhase::Before,
                            source,
                        },
                    };
                    return Err(rollback(&mut *transaction, original));
                }
            }

            let result = match operation {
                Operation::Get { key } => transaction.get(key),
                Operation::Put { key, value } => transaction.put(key, value),
                Operation::Delete { key } => transaction.delete(key),
            };
            let result = match result {
                Ok(value) => value,
                Err(source) => {
                    let original = ServiceError::Operation {
                        operation: index,
                        source: OperationFailure::Storage(source),
                    };
                    return Err(rollback(&mut *transaction, original));
                }
            };
            results.push(result);

            for hook in middleware.iter_mut().rev() {
                if let Err(source) = hook.handle(MiddlewarePhase::After, index, operation) {
                    let original = ServiceError::Operation {
                        operation: index,
                        source: OperationFailure::Middleware {
                            phase: MiddlewarePhase::After,
                            source,
                        },
                    };
                    return Err(rollback(&mut *transaction, original));
                }
            }
        }

        if let Err(source) = transaction.commit() {
            return Err(rollback(&mut *transaction, ServiceError::Commit(source)));
        }
        Ok(results)
    }
}

fn validate(operations: &[Operation]) -> Result<(), ServiceError> {
    for (index, operation) in operations.iter().enumerate() {
        if !valid_key(operation.key()) {
            return Err(ServiceError::InvalidKey {
                operation: index,
                key: operation.key().to_owned(),
            });
        }
    }
    Ok(())
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn rollback(transaction: &mut dyn Transaction, original: ServiceError) -> ServiceError {
    match transaction.rollback() {
        Ok(()) => original,
        Err(rollback) => ServiceError::Rollback {
            original: Box::new(original),
            rollback,
        },
    }
}
