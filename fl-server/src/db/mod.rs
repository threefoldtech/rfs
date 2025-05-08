pub mod map;
pub mod sqlite;
use crate::models::User;

pub trait DB: Send + Sync {
    async fn get_user_by_username(&self, username: &str) -> Option<User>;
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
}
