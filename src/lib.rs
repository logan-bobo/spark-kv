#![deny(missing_docs)]

//! a simple implementation of a key value store that supports
//! key value setting, retrival and removal.

use failure::{format_err, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, SeekFrom};
use std::io::{BufReader, Seek, Write};
use std::ops::Add;
use std::path::PathBuf;

/// wrap a generic return type with a dynamic error
pub type Result<T> = std::result::Result<T, Error>;

/// [KvStore] allows for the persistence of key value pairs to a WAL
/// with fast retrival via an in memory index.
#[derive(Debug)]
pub struct KvStore {
    data: HashMap<String, u64>,
    wal: Wal,
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
            wal: Wal::new(file, 0),
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

        self.wal.file.write_all(serialized_command.as_bytes())?;
        self.wal.file.flush()?;

        // the write marker is the first byte of the command
        self.data.insert(key, self.wal.write_marker);

        self.wal.write_marker = self
            .wal
            .write_marker
            .add(serialized_command.as_bytes().len() as u64);

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
        match self.data.get(&key) {
            Some(log_pointer) => {
                self.wal.file.seek(SeekFrom::Start(*log_pointer))?;

                let mut reader = BufReader::new(&mut self.wal.file);

                let mut line = String::new();
                let _ = reader.read_line(&mut line);

                let wal_command = serde_json::from_str::<WalCommand>(&line)?;

                Ok(wal_command.value)
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

                self.wal.file.write_all(serialized_command.as_bytes())?;
                self.wal.file.flush()?;
                self.data.remove(&key);
            }
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

        let mut reader = BufReader::new(&mut kv_store.wal.file);
        let mut line = String::new();

        while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 {
                break;
            }

            let position = reader.stream_position()? - bytes as u64;

            let wal_comnmand = serde_json::from_str::<WalCommand>(&line)?;

            match wal_comnmand.action {
                KvAction::Set => {
                    kv_store.data.insert(wal_comnmand.key, position);
                }
                KvAction::Rm => {
                    kv_store.data.remove(&wal_comnmand.key);
                }
                KvAction::Get => {}
            }

            line.clear();
        }

        kv_store.wal.write_marker = reader.stream_position()?;

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

#[derive(Debug)]
struct Wal {
    file: File,
    write_marker: u64,
}

impl Wal {
    fn new(file: File, write_marker: u64) -> Self {
        Self { file, write_marker }
    }
}
