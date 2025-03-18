use std::collections::HashMap;

pub struct KvStore {
    data: HashMap<String, String>,
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: String) -> Option<String> {
        self.data.get(&key).cloned()
    }

    pub fn remove(&mut self, key: String) {
        self.data.remove(&key);
    }
}
