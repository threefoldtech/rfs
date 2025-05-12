use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Block {
    pub index: u64,    // The index of the block in the file
    pub hash: String,  // The hash of the block's content
    pub data: Vec<u8>, // The actual data of the block
    pub size: usize,   // The size of the block's data
}

impl Block {
    /// Calculates the hash of the block's data using SHA-256.
    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
