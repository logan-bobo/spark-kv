#![deny(missing_docs)]

//! a simple implementation of a key value store in memory that supports
//! key value setting, retrival and removal.

use std::collections::HashMap;

/// [KvStore] holds key value pairs in memory that have set, get and removal
/// methods available
pub struct KvStore {
    data: HashMap<String, String>,
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KvStore {
    /// provides a new instance of a [KvStore]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let kv = KvStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// allows a caller to set a new unique key
    /// if the key already exists the value is overwritten
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut kv = KvStore::new();
    /// kv.set("Key1".to_string(), "Val1".to_string());
    /// let value1 = kv.get("Key1".to_string());
    ///
    /// assert_eq!(value1, Some("Val1".to_string()));
    /// ```
    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    /// allows a caller to retrieve a value for a given key
    /// if the key exists the value is `Some(value)` or
    /// if the key does not exists `None` is returned
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut kv = KvStore::new();
    /// kv.set("Key1".to_string(), "Val1".to_string());
    ///
    /// let value1 = kv.get("Key1".to_string());
    /// let no_value = kv.get("NoKey".to_string());
    /// #
    /// assert_eq!(value1, Some("Val1".to_string()));
    /// assert_eq!(no_value, None);
    /// ```
    pub fn get(&self, key: String) -> Option<String> {
        self.data.get(&key).cloned()
    }

    /// removes a given key, if the key does not exist
    /// no key is removed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    ///
    /// let mut kv = KvStore::new();
    /// kv.set("Key1".to_string(), "Val1".to_string());
    /// kv.remove("Key1".to_string());
    /// let value1 = kv.get("Key1".to_string());
    ///
    /// assert_eq!(value1, None);
    /// ```
    pub fn remove(&mut self, key: String) {
        self.data.remove(&key);
    }
}
