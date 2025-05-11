use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct File {
    pub id: i64,           // Auto-incrementing identifier for the file
    pub file_name: String, // Name of the file
    pub file_hash: String, // Hash of the file content
}

impl File {
    /// Creates a new file with the given file name and file hash.
    /// The id will be assigned by the database.
    pub fn new(file_name: String, file_hash: String) -> Self {
        Self {
            id: 0, // This will be replaced by the database
            file_name,
            file_hash,
        }
    }
}
