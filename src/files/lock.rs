use p2panda_rs::entry::traits::AsEncodedEntry;
use p2panda_rs::entry::EncodedEntry;
use p2panda_rs::hash::Hash;
use p2panda_rs::operation::EncodedOperation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub entry_hash: Hash,
    pub entry: EncodedEntry,
    pub operation: EncodedOperation,
}

impl Commit {
    pub fn new(entry: &EncodedEntry, operation: &EncodedOperation) -> Self {
        Self {
            entry_hash: entry.hash(),
            entry: entry.clone(),
            operation: operation.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockFile {
    pub version: u64,
    pub commits: Vec<Commit>,
}

impl LockFile {
    pub fn new(commits: Vec<Commit>) -> Self {
        Self {
            version: 1,
            commits,
        }
    }
}
