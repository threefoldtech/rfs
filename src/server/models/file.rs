use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct File {
    pub file_hash: String,     // Hash of the file content
    pub file_content: Vec<u8>, // Content of the file
}

impl File {
    /// Calculates the hash of the block's data using SHA-256.
    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Creates a new File instance by calculating the hash of the content.
    pub fn new(file_content: Vec<u8>) -> Self {
        let file_hash = Self::calculate_hash(&file_content);
        Self {
            file_hash,
            file_content,
        }
    }
}
