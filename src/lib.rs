#![deny(missing_docs)]

//! a simple implementation of a key value store that supports
//! key value setting, retrival and removal.

use failure::{format_err, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;

/// wrap a generic return type with a dynamic error
pub type Result<T> = std::result::Result<T, Error>;

/// [KvStore] allows for the persistence of key value pairs to a WAL
/// with fast retrival via an in memory index.
pub struct KvStore {
    data: HashMap<String, usize>,
    wal: File,
}

impl KvStore {
    /// provides a new instance of a [KvStore], this requires
    /// a file to ready and write to that is the write ahead log
    /// known as a WAL
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// use tempfile::tempfile;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    ///
    /// let file = tempfile()?;
    ///
    /// let kv = KvStore::new(file);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(file: File) -> Self {
        Self {
            data: HashMap::new(),
            wal: file,
        }
    }

    /// set a new unique key
    /// if the key already exists the value is overwritten
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// use tempfile::tempfile;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    ///
    /// let file = tempfile()?;
    /// let mut kv = KvStore::new(file);
    ///
    /// kv.set("Key1".to_string(), "Val1".to_string());
    ///
    /// let value1 = kv.get("Key1".to_string())?;
    ///
    /// assert_eq!(value1, Some("Val1".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let mut serialized_command = serde_json::to_string(&WalCommand::new(
            KvAction::Set,
            key.clone(),
            Some(value.clone()),
        ))?;

        serialized_command.push('\n');

        self.wal.write_all(serialized_command.as_bytes())?;
        self.wal.flush()?;

        // this is a signal that I need to find a better way to track where
        // data is being written and read. Reading the whole file and itterating
        // using a line as the offset is not a good idea at all
        self.wal.seek(std::io::SeekFrom::Start(0))?;
        let mut wal_data = String::new();
        let _ = self.wal.read_to_string(&mut wal_data);

        let wal_commands: Vec<&str> = wal_data.lines().collect();

        // the cursor tracking the file location trats the first line as 0
        self.data.insert(key, wal_commands.len() - 1);

        Ok(())
    }

    /// retrieve a value for a given key
    /// if the key exists the value is `Some(value)` or
    /// if the key does not exists `None` is returned
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// use tempfile::tempfile;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    ///
    /// let file = tempfile()?;
    /// let mut kv = KvStore::new(file);
    ///
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
        self.wal.seek(std::io::SeekFrom::Start(0))?;

        match self.data.get(&key) {
            Some(log_pointer) => {
                let mut wal_data = String::new();

                self.wal.read_to_string(&mut wal_data)?;

                if let Some(line) = wal_data.lines().nth(*log_pointer) {
                    let command = serde_json::from_str::<WalCommand>(line)?;
                    Ok(Some(match command.value {
                        Some(value) => value,
                        None => {
                            return Err(format_err!(
                                "index error: index points to a key with no value"
                            ));
                        }
                    }))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// removes a given key, if the key does not exist
    /// no key is removed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kvs::KvStore;
    /// use tempfile::tempfile;
    /// # use kvs::Result;
    /// # fn main() -> Result<()> {
    ///
    /// let file = tempfile()?;
    /// let mut kv = KvStore::new(file);
    ///
    /// kv.set("Key1".to_string(), "Val1".to_string())?;
    /// kv.remove("Key1".to_string());
    ///
    /// let value1 = kv.get("Key1".to_string())?;
    ///
    /// assert_eq!(value1, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove(&mut self, key: String) -> Result<()> {
        match self.data.get(&key) {
            Some(_) => {
                let mut serialized_command =
                    serde_json::to_string(&WalCommand::new(KvAction::Rm, key.clone(), None))?;

                serialized_command.push('\n');

                self.wal.write_all(serialized_command.as_bytes())?;
                self.wal.flush()?;
                self.data.remove(&key);
            }
            // TODO: get rid of failure crate and use anyhow
            None => return Err(format_err!("Key not found")),
        }

        Ok(())
    }

    /// opens a given path and creates the DB file if it does
    /// not exist this will be the persistent storage of the WAL
    /// replaying that wall to build an in memory index
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let mut path: PathBuf = path.into();
        path.push("kvs.db");

        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)?;

        let mut kv_store = KvStore::new(file);

        let mut wal_data = String::new();

        let _ = kv_store.wal.read_to_string(&mut wal_data)?;

        for (index, line) in wal_data.lines().enumerate() {
            let command = serde_json::from_str::<WalCommand>(line)?;
            match command.action {
                KvAction::Set => {
                    kv_store.data.insert(command.key, index);
                }
                KvAction::Rm => {
                    kv_store.data.remove(&command.key);
                }
                KvAction::Get => continue,
            }
        }

        Ok(kv_store)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WalCommand {
    action: KvAction,
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

impl WalCommand {
    fn new(action: KvAction, key: String, value: Option<String>) -> Self {
        Self { action, key, value }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum KvAction {
    Set,
    Get,
    Rm,
}
