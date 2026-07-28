#![deny(missing_docs)]

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek};
use std::path::{Path, PathBuf};

use crate::wal::{KvAction, Wal, WalCommand};

/// wrap a generic return type with a domain error
pub type Result<T> = std::result::Result<T, KvError>;

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
    pub fn new(base_path: PathBuf, data_file: File) -> Self {
        Self {
            index: HashMap::new(),
            wal: Wal::new(base_path, data_file, 0),
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
        let wal_command = WalCommand::new(KvAction::Set, key.clone(), Some(value));

        let command_first_byte = self.wal.write(wal_command)?;

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
            None => return Err(KvError::KeyNotFound),
        }

        Ok(())
    }

    /// opens a given path and creates the DB file if it does
    /// not exist this will be created writing the contents
    /// of the DB to the in memory index.
    pub fn open(path: &Path) -> Result<KvStore> {
        let data_path = return_next_or_current_file(path)?;

        let current_data_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&data_path)?;

        let mut kv_store = KvStore::new(current_data_file);

        let next_write_location = kv_store.build_index(&path)?;

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

            match wal_comnmand.action {
                KvAction::Set => {
                    self.index.insert(wal_comnmand.key, position);
                }
                KvAction::Rm => {
                    self.index.remove(&wal_comnmand.key);
                }
                KvAction::Get => {}
            }

            line.clear();
        }

        Ok(reader.stream_position()?)
    }
}

// TODO: The wal should own this one!
fn return_next_or_current_file(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        return Err(KvError::DirectoryError);
    }

    let read_dir = fs::read_dir(path)?;

    let mut watermark = 1;

    for file in read_dir {
        let file = file?;
        if file.path().is_dir() {
            continue;
        } else {
            if file.file_name().to_string_lossy().contains("kvs") && file.metadata()?.len() > 1000 {
                watermark += 1;
            }
        }
    }

    let mut data_path: PathBuf = PathBuf::from(path);

    data_path.push(format!("kvs-{}.db", watermark.to_string()));

    Ok(data_path)
}

#[derive(thiserror::Error, Debug)]
/// [KvError] represents the failure modes of the Kv server and Wal file it controlls
pub enum KvError {
    /// The key directory is not a valid directory, a [KvStore]
    /// must be opened in a directory
    #[error("invalid directory")]
    DirectoryError,

    /// The key could not be found in the index or as a Wal entry
    #[error("Key not found")]
    KeyNotFound,

    /// Internal I/O error when interacting with the Wal
    #[error("I/O error: {0}")]
    InternalIOError(#[from] std::io::Error),

    /// Faulure to deserialize the given commend from the Wal
    #[error("Could not deserialize command: {0}")]
    CommandDeserializationError(#[from] serde_json::Error),
}
