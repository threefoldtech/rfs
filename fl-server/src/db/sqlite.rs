use super::{storage::Storage, DB};
use crate::models::{File, User};
use anyhow::Result;
use sqlx::{query, query_as, Row, SqlitePool};

#[derive(Debug)]
pub struct SqlDB {
    pool: SqlitePool, // Use a connection pool for efficient database access
    storage: Storage, // Directory for storing blocks
}

static SCHEMA: &str = include_str!("../../schema/schema.sql");

impl SqlDB {
    pub async fn new(database_filepath: &str, storage_dir: &str) -> Self {
        // Check if the database file exists, and create it if it doesn't
        if !std::path::Path::new(database_filepath).exists() {
            std::fs::File::create(database_filepath).expect("Failed to create database file");
        }

        let pool = SqlitePool::connect_lazy(database_filepath)
            .expect("Failed to create database connection pool");

        // Initialize the database schema
        Self::init_schema(&pool)
            .await
            .expect("Failed to initialize database schema");

        let storage = Storage::new(storage_dir);
        Self { pool, storage }
    }

    /// Initialize the database schema
    async fn init_schema(pool: &SqlitePool) -> Result<(), anyhow::Error> {
        sqlx::query(SCHEMA)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create database schema: {}", e))?;

        log::info!("Database schema initialized successfully");
        Ok(())
    }

    async fn metadata_exists(&self, file_hash: &str, block_index: u64, block_hash: &str) -> bool {
        let result = query(
            "SELECT COUNT(*) as count FROM metadata WHERE file_hash = ? AND block_index = ? AND block_hash = ?",
        )
        .bind(file_hash)
        .bind(block_index as i64)
        .bind(block_hash)
        .fetch_one(&self.pool);

        match result.await {
            Ok(row) => {
                let count: i64 = row.get(0);
                count > 0
            }
            Err(err) => {
                log::error!("Error checking if metadata exists: {}", err);
                false
            }
        }
    }
}

impl DB for SqlDB {
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        let query = "SELECT * FROM users WHERE username = ?";
        let result = query_as::<_, User>(query)
            .bind(username)
            .fetch_one(&self.pool);

        match result.await {
            Ok(user) => Some(user),
            Err(_) => None,
        }
    }

    async fn block_exists(&self, file_hash: &str, block_index: u64, block_hash: &str) -> bool {
        // Check if the block already exists in storage
        let block_exists = self.storage.block_exists(block_hash);

        // Check if the metadata already exists in the database
        let metadata_exists = self
            .metadata_exists(file_hash, block_index, block_hash)
            .await;

        // If both block and metadata exist, no need to store again
        if block_exists && (metadata_exists || file_hash.is_empty()) {
            return true;
        }

        false // Block does not exist
    }

    async fn store_block(
        &self,
        block_hash: &str,
        data: Vec<u8>,
        file_hash: &str,
        block_index: u64,
    ) -> Result<bool, anyhow::Error> {
        // Check if the block already exists in storage
        let block_exists = self.storage.block_exists(block_hash);

        // Check if the metadata already exists in the database
        let metadata_exists = self
            .metadata_exists(file_hash, block_index, block_hash)
            .await;

        // If both block and metadata exist, no need to store again
        if block_exists && (metadata_exists || file_hash.is_empty()) {
            return Ok(false);
        }

        // Store metadata if it doesn't exist
        if !metadata_exists {
            if let Err(err) = query(
                "INSERT INTO metadata (file_hash, block_index, block_hash, created_at) 
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
            )
            .bind(file_hash)
            .bind(block_index as i64)
            .bind(block_hash)
            .execute(&self.pool)
            .await
            {
                log::error!("Error storing metadata: {}", err);
                return Err(anyhow::anyhow!("Failed to store metadata: {}", err));
            }
        }

        // Store the block data in the file system if it doesn't exist
        if !block_exists {
            if let Err(err) = self.storage.save_block(block_hash, &data) {
                log::error!("Error storing block in storage: {}", err);
                return Err(anyhow::anyhow!("Failed to store block in storage: {}", err));
            }
        }

        Ok(true) // Indicate that the block or metadata was newly stored
    }

    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        // Retrieve the block data from storage
        match self.storage.get_block(hash) {
            Ok(Some(data)) => Ok(Some(data)),
            Ok(None) => Ok(None),
            Err(err) => {
                log::error!("Error retrieving block from storage: {}", err);
                Err(anyhow::anyhow!(
                    "Failed to retrieve block from storage: {}",
                    err
                ))
            }
        }
    }

    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<File>, anyhow::Error> {
        // Retrieve the blocks associated with the file hash
        let blocks = match self.get_file_blocks_ordered(hash).await {
            Ok(blocks) => blocks,
            Err(err) => {
                log::error!("Failed to retrieve file blocks: {}", err);
                return Err(anyhow::anyhow!("Failed to retrieve file blocks: {}", err));
            }
        };

        if blocks.is_empty() {
            return Ok(None); // No blocks found, file does not exist
        }

        // Combine block data to reconstruct the file
        let mut file_content = Vec::new();
        for (block_hash, _) in blocks {
            match self.storage.get_block(&block_hash) {
                Ok(Some(data)) => file_content.extend(data),
                Ok(None) => {
                    log::error!("Block {} not found", block_hash);
                    return Err(anyhow::anyhow!("Block {} not found", block_hash));
                }
                Err(err) => {
                    log::error!("Failed to retrieve block {}: {}", block_hash, err);
                    return Err(anyhow::anyhow!(
                        "Failed to retrieve block {}: {}",
                        block_hash,
                        err
                    ));
                }
            }
        }

        // Return the reconstructed file
        Ok(Some(File {
            file_hash: hash.to_string(),
            file_content,
        }))
    }

    async fn get_file_blocks_ordered(
        &self,
        file_hash: &str,
    ) -> Result<Vec<(String, u64)>, anyhow::Error> {
        let result = query(
            "SELECT block_hash, block_index FROM metadata WHERE file_hash = ? ORDER BY block_index",
        )
        .bind(file_hash)
        .fetch_all(&self.pool)
        .await;

        match result {
            Ok(rows) => {
                let blocks = rows
                    .into_iter()
                    .map(|row| {
                        let block_hash: String = row.get(0);
                        let block_index: i64 = row.get(1);
                        (block_hash, block_index as u64)
                    })
                    .collect::<Vec<(String, u64)>>();

                Ok(blocks)
            }
            Err(err) => {
                log::error!("Error retrieving file blocks: {}", err);
                Err(anyhow::anyhow!("Failed to retrieve file blocks: {}", err))
            }
        }
    }
}
