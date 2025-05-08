use super::DB;
use crate::models::User;
use sqlx::{query_as, SqlitePool};

#[derive(Debug)]
pub struct SqlDB {
    pool: SqlitePool, // Use a connection pool for efficient database access
}

impl SqlDB {
    pub fn new(database_filepath: &str) -> Self {
        let pool = SqlitePool::connect_lazy(database_filepath)
            .expect("Failed to create database connection pool");
        Self { pool }
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
}
