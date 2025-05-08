pub mod map;
pub mod sqlite;
use crate::models::User;

pub trait DB: Send + Sync {
    async fn get_user_by_username(&self, username: &str) -> Option<User>;
    async fn block_exists(&self, hash: &str) -> bool;
    async fn store_block(&self, hash: &str, data: Vec<u8>) -> Result<bool, anyhow::Error>;
    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error>;
}

pub enum DBType {
    MapDB(map::MapDB),
    SqlDB(sqlite::SqlDB),
}

impl DB for DBType {
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        match self {
            DBType::MapDB(db) => db.get_user_by_username(username).await,
            DBType::SqlDB(db) => db.get_user_by_username(username).await,
        }
    }

    async fn block_exists(&self, hash: &str) -> bool {
        match self {
            DBType::MapDB(db) => db.block_exists(hash).await,
            DBType::SqlDB(db) => db.block_exists(hash).await,
        }
    }

    async fn store_block(&self, hash: &str, data: Vec<u8>) -> Result<bool, anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.store_block(hash, data).await,
            DBType::SqlDB(db) => db.store_block(hash, data).await,
        }
    }

    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        match self {
            DBType::MapDB(db) => db.get_block(hash).await,
            DBType::SqlDB(db) => db.get_block(hash).await,
        }
    }
}
