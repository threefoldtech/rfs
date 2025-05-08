use super::DB;
use crate::models::User;
use anyhow::Result;
use sqlx::{query, query_as, Row, SqlitePool};

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
        // Create blocks table if it doesn't exist
        let schema = r#"
        CREATE TABLE IF NOT EXISTS blocks (
            hash VARCHAR(64) PRIMARY KEY,
            data BLOB NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
        
        CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks (hash);
        "#;

        sqlx::query(schema)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create blocks table: {}", e))?;

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

    async fn store_block(&self, hash: &str, data: Vec<u8>) -> Result<bool, anyhow::Error> {
        // First check if the block already exists
        let exists = self.block_exists(hash).await;

        if exists {
            return Ok(false); // Block already exists, not newly stored
        }

        // Insert the new block with the provided data
        let result = query("INSERT INTO blocks (hash, data) VALUES (?, ?)")
            .bind(hash)
            .bind(&data)
            .execute(&self.pool)
            .await;

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
}
