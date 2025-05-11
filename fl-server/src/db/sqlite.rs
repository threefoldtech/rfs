use super::DB;
use crate::models::{File, User};
use anyhow::Result;
use sqlx::{query, query_as, Row, Sqlite, SqlitePool, Transaction};

#[derive(Debug)]
pub struct SqlDB {
    pool: SqlitePool, // Use a connection pool for efficient database access
}

impl SqlDB {
    pub fn new(database_filepath: &str) -> Self {
        let pool = SqlitePool::connect_lazy(database_filepath)
            .expect("Failed to create database connection pool");

        // Create a runtime to execute the initialization
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

        // Initialize the database schema
        rt.block_on(async {
            Self::init_schema(&pool)
                .await
                .expect("Failed to initialize database schema");
        });

        Self { pool }
    }

    /// Initialize the database schema
    async fn init_schema(pool: &SqlitePool) -> Result<(), anyhow::Error> {
        // Create blocks and files tables if they don't exist
        let schema = r#"
        -- Table to store blocks with their hash and data
        CREATE TABLE IF NOT EXISTS blocks (
            hash VARCHAR(64) PRIMARY KEY,
            data BLOB NOT NULL,
            file_hash VARCHAR(64),
            block_index INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks (hash);
        CREATE INDEX IF NOT EXISTS idx_blocks_file_hash ON blocks (file_hash);
        
        -- Table to store file metadata
        CREATE TABLE IF NOT EXISTS files (
            id VARCHAR(36) PRIMARY KEY,
            file_name VARCHAR(255) NOT NULL,
            file_hash VARCHAR(64) NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE INDEX IF NOT EXISTS idx_files_hash ON files (file_hash);
        "#;

        sqlx::query(schema)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create database schema: {}", e))?;

        log::info!("Database schema initialized successfully");
        Ok(())
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

    async fn block_exists(&self, hash: &str) -> bool {
        let result = query("SELECT COUNT(*) as count FROM blocks WHERE hash = ?")
            .bind(hash)
            .fetch_one(&self.pool);

        match result.await {
            Ok(row) => {
                let count: i64 = row.get(0);
                count > 0
            }
            Err(err) => {
                log::error!("Error checking if block exists: {}", err);
                false
            }
        }
    }

    async fn store_block(
        &self,
        hash: &str,
        data: Vec<u8>,
        file_hash: Option<String>,
        block_index: Option<u64>,
    ) -> Result<bool, anyhow::Error> {
        // First check if the block already exists
        let exists = self.block_exists(hash).await;

        if exists {
            return Ok(false); // Block already exists, not newly stored
        }

        // Insert the new block with the provided data
        let result = match (file_hash, block_index) {
            (Some(fh), Some(idx)) => {
                query("INSERT INTO blocks (hash, data, file_hash, block_index) VALUES (?, ?, ?, ?)")
                    .bind(hash)
                    .bind(&data)
                    .bind(fh)
                    .bind(idx as i64)
                    .execute(&self.pool)
                    .await
            }
            _ => {
                query("INSERT INTO blocks (hash, data) VALUES (?, ?)")
                    .bind(hash)
                    .bind(&data)
                    .execute(&self.pool)
                    .await
            }
        };

        match result {
            Ok(_) => Ok(true), // Block was newly stored
            Err(err) => {
                log::error!("Error storing block: {}", err);
                Err(anyhow::anyhow!("Failed to store block: {}", err))
            }
        }
    }

    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let result = query("SELECT data FROM blocks WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await;

        match result {
            Ok(Some(row)) => {
                let data: Vec<u8> = row.get(0);
                Ok(Some(data))
            }
            Ok(None) => Ok(None), // Block not found
            Err(err) => {
                log::error!("Error retrieving block: {}", err);
                Err(anyhow::anyhow!("Failed to retrieve block: {}", err))
            }
        }
    }

    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<File>, anyhow::Error> {
        let result =
            query_as::<_, File>("SELECT id, file_name, file_hash FROM files WHERE file_hash = ?")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await;

        match result {
            Ok(file) => Ok(file),
            Err(err) => {
                log::error!("Error retrieving file: {}", err);
                Err(anyhow::anyhow!("Failed to retrieve file: {}", err))
            }
        }
    }

    async fn get_file_blocks(&self, file_hash: &str) -> Result<Vec<(String, u64)>, anyhow::Error> {
        let result =
            query("SELECT hash, block_index FROM blocks WHERE file_hash = ? ORDER BY block_index")
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
                    .collect();

                Ok(blocks)
            }
            Err(err) => {
                log::error!("Error retrieving file blocks: {}", err);
                Err(anyhow::anyhow!("Failed to retrieve file blocks: {}", err))
            }
        }
    }
}
