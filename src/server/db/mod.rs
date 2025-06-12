pub mod map;
pub mod sqlite;
mod storage;
use crate::server::models::{File, User};

pub trait DB: Send + Sync {
    // User methods
    async fn get_user_by_username(&self, username: &str) -> Option<User>;

    // Block methods
    async fn block_exists(
        &self,
        file_hash: &str,
        block_index: u64,
        block_hash: &str,
        user_id: i64,
    ) -> bool;
    async fn store_block(
        &self,
        block_hash: &str,
        data: Vec<u8>,
        file_hash: &str,
        block_index: u64,
        user_id: i64,
    ) -> Result<bool, anyhow::Error>;
    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error>;
    async fn increment_block_downloads(&self, hash: &str) -> Result<(), anyhow::Error>;
    async fn get_block_downloads(&self, hash: &str) -> Result<(u64, u64), anyhow::Error>;

    // File methods
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<File>, anyhow::Error>;
    async fn get_file_blocks_ordered(
        &self,
        file_hash: &str,
    ) -> Result<Vec<(String, u64)>, anyhow::Error>;
    async fn list_blocks(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<String>, u64), anyhow::Error>;

    // Get all blocks related to a user
    async fn get_user_blocks(&self, user_id: i64) -> Result<Vec<(String, u64)>, anyhow::Error>;
}

pub enum DBType {
    MapDB(map::MapDB),
    SqlDB(sqlite::SqlDB),
}

impl DB for DBType {
    // User methods
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        match self {
            DBType::MapDB(db) => db.get_user_by_username(username).await,
            DBType::SqlDB(db) => db.get_user_by_username(username).await,
        }
    }

    // Block methods
    async fn block_exists(
        &self,
        file_hash: &str,
        block_index: u64,
        block_hash: &str,
        user_id: i64,
    ) -> bool {
        match self {
            DBType::MapDB(db) => {
                db.block_exists(file_hash, block_index, block_hash, user_id)
                    .await
            }
            DBType::SqlDB(db) => {
                db.block_exists(file_hash, block_index, block_hash, user_id)
                    .await
            }
        }
    }

    async fn store_block(
        &self,
        block_hash: &str,
        data: Vec<u8>,
        file_hash: &str,
        block_index: u64,
        user_id: i64,
    ) -> Result<bool, anyhow::Error> {
        match self {
            DBType::MapDB(db) => {
                db.store_block(block_hash, data, file_hash, block_index, user_id)
                    .await
            }
            DBType::SqlDB(db) => {
                db.store_block(block_hash, data, file_hash, block_index, user_id)
                    .await
            }
        }
    }

    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.get_block(hash).await,
            DBType::SqlDB(db) => db.get_block(hash).await,
        }
    }

    async fn increment_block_downloads(&self, hash: &str) -> Result<(), anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.increment_block_downloads(hash).await,
            DBType::SqlDB(db) => db.increment_block_downloads(hash).await,
        }
    }

    async fn get_block_downloads(&self, hash: &str) -> Result<(u64, u64), anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.get_block_downloads(hash).await,
            DBType::SqlDB(db) => db.get_block_downloads(hash).await,
        }
    }

    // File methods
    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<File>, anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.get_file_by_hash(hash).await,
            DBType::SqlDB(db) => db.get_file_by_hash(hash).await,
        }
    }

    async fn get_file_blocks_ordered(
        &self,
        file_hash: &str,
    ) -> Result<Vec<(String, u64)>, anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.get_file_blocks_ordered(file_hash).await,
            DBType::SqlDB(db) => db.get_file_blocks_ordered(file_hash).await,
        }
    }

    async fn list_blocks(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<String>, u64), anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.list_blocks(page, per_page).await,
            DBType::SqlDB(db) => db.list_blocks(page, per_page).await,
        }
    }

    async fn get_user_blocks(&self, user_id: i64) -> Result<Vec<(String, u64)>, anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.get_user_blocks(user_id).await,
            DBType::SqlDB(db) => db.get_user_blocks(user_id).await,
        }
    }
}
