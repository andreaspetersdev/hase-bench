use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::StorageError;

pub trait Transaction {
    fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&mut self, key: &str, value: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    fn delete(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn commit(&mut self) -> Result<(), StorageError>;
    fn rollback(&mut self) -> Result<(), StorageError>;
}

pub trait Storage {
    fn begin(&mut self) -> Result<Box<dyn Transaction + '_>, StorageError>;
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    values: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryStore {
    pub fn from_entries(entries: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        Self {
            values: Arc::new(Mutex::new(entries.into_iter().collect())),
        }
    }

    pub fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        lock(&self.values).clone()
    }
}

impl Storage for MemoryStore {
    fn begin(&mut self) -> Result<Box<dyn Transaction + '_>, StorageError> {
        let staged = lock(&self.values).clone();
        Ok(Box::new(MemoryTransaction {
            values: Arc::clone(&self.values),
            staged,
            closed: false,
        }))
    }
}

struct MemoryTransaction {
    values: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    staged: BTreeMap<String, Vec<u8>>,
    closed: bool,
}

impl MemoryTransaction {
    fn ensure_open(&self) -> Result<(), StorageError> {
        if self.closed {
            Err(StorageError::TransactionClosed)
        } else {
            Ok(())
        }
    }
}

impl Transaction for MemoryTransaction {
    fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.ensure_open()?;
        Ok(self.staged.get(key).cloned())
    }

    fn put(&mut self, key: &str, value: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        self.ensure_open()?;
        Ok(self.staged.insert(key.to_owned(), value.to_vec()))
    }

    fn delete(&mut self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        self.ensure_open()?;
        Ok(self.staged.remove(key))
    }

    fn commit(&mut self) -> Result<(), StorageError> {
        self.ensure_open()?;
        *lock(&self.values) = self.staged.clone();
        self.closed = true;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), StorageError> {
        self.ensure_open()?;
        self.closed = true;
        Ok(())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
