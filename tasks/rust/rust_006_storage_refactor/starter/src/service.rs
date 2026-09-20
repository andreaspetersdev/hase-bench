use crate::model::{Operation, ServiceError};
use crate::storage::MemoryStore;

pub struct DocumentService {
    storage: MemoryStore,
}

impl DocumentService {
    pub fn new(storage: MemoryStore) -> Self {
        Self { storage }
    }

    pub fn execute(
        &mut self,
        operations: Vec<Operation>,
    ) -> Result<Vec<Option<Vec<u8>>>, ServiceError> {
        let mut results = Vec::with_capacity(operations.len());
        for (index, operation) in operations.into_iter().enumerate() {
            let key = operation.key();
            if !valid_key(key) {
                return Err(ServiceError::InvalidKey {
                    operation: index,
                    key: key.to_owned(),
                });
            }
            let result = match operation {
                Operation::Get { key } => self.storage.get(&key),
                Operation::Put { key, value } => self.storage.put(key, value),
                Operation::Delete { key } => self.storage.delete(&key),
            };
            results.push(result);
        }
        Ok(results)
    }
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}
