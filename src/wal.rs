#![deny(missing_docs)]

use failure::Error;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub enum KvAction {
    Set,
    Get,
    Rm,
}

#[derive(Debug)]
pub struct Wal {
    pub file: File,
    write_marker: u64,
}

impl Wal {
    pub fn new(file: File, write_marker: u64) -> Self {
        Self { file, write_marker }
    }

    pub fn write(&mut self, command: WalCommand) -> Result<u64, Error> {
        let mut serialized_command = serde_json::to_string(&command)?;
        serialized_command.push('\n');

        self.file.write_all(serialized_command.as_bytes())?;
        self.file.flush()?;

        Ok(serialized_command.len() as u64)
    }

    pub fn get_write_marker(&self) -> u64 {
        self.write_marker
    }

    pub fn set_write_marker_possition(&mut self, byte: u64) {
        self.write_marker = byte;
    }

    pub fn progress_write_marker(&mut self, bytes: u64) {
        self.write_marker += bytes;
    }

    pub fn get_entry(&mut self, entry_first_byte: u64) -> Result<WalCommand, Error> {
        self.file.seek(SeekFrom::Start(entry_first_byte))?;

        let mut reader = BufReader::new(&mut self.file);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);

        Ok(serde_json::from_str::<WalCommand>(&line)?)
    }
}
