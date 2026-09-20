use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

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
        self.values.lock().unwrap().clone()
    }

    pub(crate) fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.values.lock().unwrap().get(key).cloned()
    }

    pub(crate) fn put(&self, key: String, value: Vec<u8>) -> Option<Vec<u8>> {
        self.values.lock().unwrap().insert(key, value)
    }

    pub(crate) fn delete(&self, key: &str) -> Option<Vec<u8>> {
        self.values.lock().unwrap().remove(key)
    }
}
