use std::collections::HashMap;
use utoipa::ToSchema;

use super::DB;
use crate::models::{File, User};
use anyhow::Result;

#[derive(Debug, ToSchema)]
pub struct MapDB {
    users: HashMap<String, User>,
}

impl MapDB {
    pub fn new(users: &[User]) -> Self {
        Self {
            users: users
                .iter()
                .map(|u| (u.username.clone(), u.to_owned()))
                .collect(),
        }
    }
}

impl DB for MapDB {
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.users.get(username).cloned()
    }

    async fn block_exists(&self, _file_hash: &str, _block_index: u64, _block_hash: &str) -> bool {
        // TODO:
        true
    }

    async fn store_block(
        &self,
        _block_hash: &str,
        _data: Vec<u8>,
        _file_hash: &str,
        _block_index: u64,
        _user_id: i64,
    ) -> Result<bool, anyhow::Error> {
        // TODO: Implement block storage logic
        Ok(true) // Placeholder return value
    }

    async fn get_block(&self, _hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        // TODO:
        Ok(None)
    }

    async fn get_file_by_hash(&self, _hash: &str) -> Result<Option<File>, anyhow::Error> {
        // TODO:
        Ok(None)
    }

    async fn get_file_blocks_ordered(
        &self,
        _file_hash: &str,
    ) -> Result<Vec<(String, u64)>, anyhow::Error> {
        // TODO:
        Ok(Vec::new())
    }

    async fn list_blocks(
        &self,
        _page: u32,
        _per_page: u32,
    ) -> Result<(Vec<String>, u64), anyhow::Error> {
        // TODO:
        Ok((Vec::new(), 0))
    }

    async fn get_user_blocks(&self, _user_id: i64) -> Result<Vec<(String, u64)>, anyhow::Error> {
        // TODO:
        Ok(Vec::new())
    }

    async fn increment_block_downloads(&self, _hash: &str) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn get_block_downloads(&self, _hash: &str) -> Result<(u64, u64), anyhow::Error> {
        Ok((0, 0))
    }
}
