#![deny(missing_docs)]

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::{Path, PathBuf};

use failure::{format_err, Error};

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
    pub fn new(data_file: File) -> Self {
        Self {
            index: HashMap::new(),
            wal: Wal::new(data_file, 0),
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
        let command_first_byte = self.wal.write(WalCommand::new(
            KvAction::Set,
            key.clone(),
            Some(value.clone()),
        ))?;

        self.index.insert(key, command_first_byte);

        if self.wal.should_compact()? {
            self.wal.compact()?;
            let stream_possition = self.build_index()?;
            self.wal.set_write_marker_possition(stream_possition);
        }

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
    pub fn open(path: &Path) -> Result<KvStore> {
        let mut data_path: PathBuf = PathBuf::from(path);
        data_path.push("kvs.db");

        let data_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&data_path)?;

        let mut kv_store = KvStore::new(data_file);

        let next_write_location = kv_store.build_index()?;

        kv_store.wal.set_write_marker_possition(next_write_location);

        Ok(kv_store)
    }

    /// Rebuild the Kv store internal index
    fn build_index(&mut self) -> Result<u64> {
        self.wal.reset_reader()?;

        let mut reader = BufReader::new(&mut self.wal.data_file);
        let mut line = String::new();

        while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 {
                break;
            }

            let position = reader.stream_position()? - bytes as u64;

            let wal_comnmand = serde_json::from_str::<WalCommand>(&line)?;

            // I might feel like I want to mess with this
            // but DONT, this is a mental note to myself
            // if you change how the index is built you could couple
            // the index to the compactor!
            //
            // For example if you rely on the fact that the compactor has removed
            // dead keys, then the compactor changes when it runs you could point to parial
            // entry. index rebuilds should be able to be called infependant of compaction.
            match wal_comnmand.action {
                KvAction::Set => {
                    self.index.insert(wal_comnmand.key, position);
                }
                KvAction::Rm => {
                    self.index.remove(&wal_comnmand.key);
                }
                // this can never happen so this to me is a signal
                // we need to seperate data manipulation actions
                // from data retrival actions... (the type is too wide)
                KvAction::Get => {}
            }

            line.clear();
        }

        Ok(reader.stream_position()?)
    }
}
