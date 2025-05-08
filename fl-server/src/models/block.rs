use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub index: u64,    // The index of the block in the file
    pub hash: String,  // The hash of the block's content
    pub data: Vec<u8>, // The actual data of the block
    pub size: usize,   // The size of the block's data
}

impl Block {
    /// Creates a new block with the given index, data, and hash.
    pub fn new(index: u64, data: Vec<u8>, hash: String) -> Self {
        let size = data.len();
        Self {
            index,
            hash,
            data,
            size,
        }
    }

    /// Calculates the hash of the block's data using SHA-256.
    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
