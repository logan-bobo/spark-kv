#![deny(missing_docs)]

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};

use crate::kv::Result;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct WalCommand {
    pub action: KvAction,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl WalCommand {
    pub fn new(action: KvAction, key: String, value: Option<String>) -> Self {
        Self { action, key, value }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum KvAction {
    Set,
    Get,
    Rm,
}

#[derive(Debug)]
pub struct Wal {
    pub data_file: File,
    write_marker: u64,
}

impl Wal {
    pub fn new(data_file: File, write_marker: u64) -> Self {
        Self {
            data_file,
            write_marker,
        }
    }

    pub fn write(&mut self, command: WalCommand) -> Result<u64> {
        let mut serialized_command = serde_json::to_string(&command)?;
        serialized_command.push('\n');

        let command_first_byte = self.get_write_marker();

        self.data_file.write_all(serialized_command.as_bytes())?;
        self.data_file.flush()?;
        self.progress_write_marker(serialized_command.len() as u64);

        Ok(command_first_byte)
    }

    fn get_write_marker(&self) -> u64 {
        self.write_marker
    }

    pub fn set_write_marker_possition(&mut self, byte: u64) {
        self.write_marker = byte;
    }

    fn progress_write_marker(&mut self, bytes: u64) {
        self.write_marker += bytes;
    }

    // Functions should never assume a reader possition
    // They should always SeekFrom the start or the first
    // byte of a command never follow on from another functions
    // reader possition
    pub fn reset_reader(&mut self) -> Result<()> {
        let _ = self.data_file.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    pub fn should_compact(&mut self) -> Result<bool> {
        Ok(self.data_file.metadata()?.len() > 1000)
    }

    pub fn get_entry(&mut self, entry_first_byte: u64) -> Result<WalCommand> {
        self.data_file.seek(SeekFrom::Start(entry_first_byte))?;

        let mut reader = BufReader::new(&mut self.data_file);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);

        Ok(serde_json::from_str::<WalCommand>(&line)?)
    }

    pub fn compact(&mut self) -> Result<()> {
        self.reset_reader()?;

        let mut new_mem_wal: HashMap<String, WalCommand> = HashMap::new();
        let mut reader = BufReader::new(&mut self.data_file);
        let mut line = String::new();

        // Build a hash set of final wal command state
        while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 {
                break;
            }

            let wal_comnmand = serde_json::from_str::<WalCommand>(&line)?;

            match wal_comnmand.action {
                KvAction::Set => {
                    new_mem_wal.insert(wal_comnmand.key.clone(), wal_comnmand);
                }
                KvAction::Rm => {
                    new_mem_wal.remove(&wal_comnmand.key);
                }
                KvAction::Get => {}
            }

            line.clear();
        }

        self.data_file.set_len(0)?;
        self.set_write_marker_possition(0);
        self.reset_reader()?;

        // what happens if we crash here
        // partial compaction on a live file...
        for command in new_mem_wal {
            self.write(command.1)?;
        }

        Ok(())
    }
}
