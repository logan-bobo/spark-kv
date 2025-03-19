#![deny(missing_docs)]

//! a simple implementation of a key value store in memory that supports
//! key value setting, retrival and removal.

use failure::Error;
use serde::Serialize;
use std::{collections::HashMap, path::Path};

/// wrap a generic return type with a dynamic error
pub type Result<T> = std::result::Result<T, Error>;

/// [KvStore] holds key value pairs in memory that have set, get and removal
/// methods available
pub struct KvStore {
    data: HashMap<String, String>,
    wal: String,
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
            wal: String::new(),
        }
    }

    /// allows a caller to set a new unique key
    /// if the key already exists the value is overwritten
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    /// let mut kv = KvStore::new();
    /// kv.set("Key1".to_string(), "Val1".to_string());
    /// let value1 = kv.get("Key1".to_string())?;
    ///
    /// assert_eq!(value1, Some("Val1".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let serialized_command =
            serde_json::to_string(&WalCommand::new(KvAction::SET, key.clone(), value.clone()))?;

        self.wal.push_str(&serialized_command);

        self.data.insert(key, value);

        println!("{:?}", self.wal);
        Ok(())
    }

    /// allows a caller to retrieve a value for a given key
    /// if the key exists the value is `Some(value)` or
    /// if the key does not exists `None` is returned
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    /// let mut kv = KvStore::new();
    /// kv.set("Key1".to_string(), "Val1".to_string())?;
    ///
    /// let value1 = kv.get("Key1".to_string())?;
    /// let no_value = kv.get("NoKey".to_string())?;
    ///
    /// assert_eq!(value1, Some("Val1".to_string()));
    /// assert_eq!(no_value, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        Ok(self.data.get(&key).cloned())
    }

    /// removes a given key, if the key does not exist
    /// no key is removed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    /// let mut kv = KvStore::new();
    /// kv.set("Key1".to_string(), "Val1".to_string())?;
    /// kv.remove("Key1".to_string());
    /// let value1 = kv.get("Key1".to_string())?;
    ///
    /// assert_eq!(value1, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove(&mut self, key: String) -> Result<()> {
        self.data.remove(&key);
        Ok(())
    }

    /// opens a given path
    pub fn open(path: &Path) -> Result<KvStore> {
        Ok(KvStore::default())
    }
}

#[derive(Debug, Serialize)]
struct WalCommand {
    action: KvAction,
    key: String,
    value: String,
}

impl WalCommand {
    fn new(action: KvAction, key: String, value: String) -> Self {
        Self { action, key, value }
    }
}

#[derive(Debug, Serialize)]
enum KvAction {
    SET,
    GET,
    RM,
}
