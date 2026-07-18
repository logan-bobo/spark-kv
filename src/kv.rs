#![deny(missing_docs)]

use failure::{format_err, Error};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::{BufReader, Seek};
use std::path::PathBuf;

use crate::wal::{KvAction, Wal, WalCommand};

/// wrap a generic return type with a dynamic error
pub type Result<T> = std::result::Result<T, Error>;

/// [KvStore] allows for the persistence of key value pairs to a WAL
/// with fast retrival via an in memory index.
#[derive(Debug)]
pub struct KvStore {
    index: HashMap<String, u64>,
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
            index: HashMap::new(),
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
        let bytes_written = self.wal.write(WalCommand::new(
            KvAction::Set,
            key.clone(),
            Some(value.clone()),
        ))?;

        // Update the index so the key points to the first
        // byte of the command
        self.index.insert(key, self.wal.get_write_marker());

        // move the write marker to the last byte of the command
        // so we can begin our next wal event
        self.wal.progress_write_marker(bytes_written);

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
        match self.index.get(&key) {
            Some(log_pointer) => {
                let result = self.wal.get_entry(*log_pointer)?;
                Ok(result.value)
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
        match self.index.get(&key) {
            Some(_) => {
                self.wal
                    .write(WalCommand::new(KvAction::Rm, key.clone(), None))?;

                self.index.remove(&key);
            }
            None => return Err(format_err!("Key not found")),
        }

        Ok(())
    }

    /// opens a given path and creates the DB file if it does
    /// not exist this will be the persistent storage of the WAL
    /// replaying that wall to build an in memory index
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let mut base_path: PathBuf = path.into();
        base_path.push("kvs.db");

        let data_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&base_path)?;

        let mut kv_store = KvStore::new(data_file);

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
                    kv_store.index.insert(wal_comnmand.key, position);
                }
                KvAction::Rm => {
                    kv_store.index.remove(&wal_comnmand.key);
                }
                // this can never happen so this to me is a signal
                // we need to seperate data manipulation actions
                // from data retrival actions... (the type is too wide)
                KvAction::Get => {}
            }

            line.clear();
        }

        // cant pass reader into the below as reader is already a
        // mutable reference
        let next_write_location = reader.stream_position()?;

        kv_store.wal.set_write_marker_possition(next_write_location);

        Ok(kv_store)
    }
}
