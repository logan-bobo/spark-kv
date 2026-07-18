#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::fs::File;

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
    pub write_marker: u64,
}

impl Wal {
    pub fn new(file: File, write_marker: u64) -> Self {
        Self { file, write_marker }
    }
}
