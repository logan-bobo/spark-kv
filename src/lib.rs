#![deny(missing_docs)]

//! a simple implementation of a key value store that supports
//! key value setting, retrival and removal.

use failure::{format_err, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
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
    pub fn new(file: File, backup_path: PathBuf, data_path: PathBuf) -> Self {
        Self {
            data: HashMap::new(),
            wal: Wal::new(file, backup_path, data_path, 0),
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
        dbg!(&self.data);
        dbg!(&self.wal.data_path);
        self.compact()?;

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
        let mut data_path: PathBuf = path.into();
        let mut backup_path = data_path.clone();

        data_path.push("kvs.db");
        backup_path.push("kvs.db.bak");

        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&data_path)?;

        let mut kv_store = KvStore::new(file, backup_path, data_path);

        kv_store.build_index()?;

        Ok(kv_store)
    }

    /// compacts the current WAL by removing dead records
    pub fn compact(&mut self) -> Result<()> {
        // wipe the current file to rebuild it from the in memory index
        // note this does not sacrifice durability as we have already
        // created a backup of the current WAL before we compact so if compaction
        // fails just rename the backup file to the main file. This is not the final
        // solution just maintains durability whilst alowing me to test compaction

        fs::File::create(&self.wal.backup_path)?;

        // before any compaction runs we need to ensure data durability
        // to do this we copy the current WAL this ensures if compaction
        // fails or corrupts the file, we can restore the database to a state
        // before the current compaction cycle
        // TODO: implement a recovery path where if kvs.db.bak exists recover
        // the wal from that file after compaction the file should not exist
        fs::copy(&self.wal.data_path, &self.wal.backup_path)?;

        self.wal.file.seek(SeekFrom::Start(0))?;

        let mut tmp_index: HashMap<String, String> = HashMap::new();
        let mut reader = BufReader::new(&mut self.wal.file);
        let mut line = String::new();

        while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 {
                break;
            }

            let wal_comnmand = serde_json::from_str::<WalCommand>(&line)?;

            dbg!(&wal_comnmand);

            match wal_comnmand.action {
                KvAction::Set => {
                    // infallible unwrap no key can exist in the wal with no value
                    tmp_index.insert(wal_comnmand.key, wal_comnmand.value.unwrap());
                }
                KvAction::Rm => {
                    tmp_index.remove(&wal_comnmand.key);
                }
                KvAction::Get => {}
            }

            line.clear();
        }

        self.wal.file.set_len(0)?;

        dbg!(&tmp_index);

        for (key, value) in tmp_index {
            let mut wal_command =
                serde_json::to_string(&WalCommand::new(KvAction::Set, key, Some(value)))?;

            wal_command.push('\n');

            self.wal.file.write_all(wal_command.as_bytes())?;
            self.wal.file.flush()?;
        }

        self.wal.file.seek(SeekFrom::Start(0))?;

        fs::remove_file(&self.wal.backup_path)?;

        Ok(())
    }

    /// rebuilds the in memory index using the current instance of a kv store
    /// by buffering each line of the WAL buidil all records have been processed
    /// maintaining the read cursor
    fn build_index(&mut self) -> Result<()> {
        let mut reader = BufReader::new(&mut self.wal.file);
        let mut line = String::new();

        while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 {
                break;
            }

            let position = reader.stream_position()? - bytes as u64;

            let wal_comnmand = serde_json::from_str::<WalCommand>(&line)?;

            match wal_comnmand.action {
                KvAction::Set => {
                    self.data.insert(wal_comnmand.key, position);
                }
                KvAction::Rm => {
                    self.data.remove(&wal_comnmand.key);
                }
                KvAction::Get => {}
            }

            line.clear();
        }

        self.wal.write_marker = reader.stream_position()?;

        Ok(())
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
    backup_path: PathBuf,
    data_path: PathBuf,
    write_marker: u64,
}

impl Wal {
    fn new(file: File, backup_path: PathBuf, data_path: PathBuf, write_marker: u64) -> Self {
        Self {
            file,
            backup_path,
            data_path,
            write_marker,
        }
    }
}
