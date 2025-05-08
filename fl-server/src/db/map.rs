use std::collections::HashMap;
use std::sync::Mutex;
use utoipa::ToSchema;

use super::DB;
use crate::models::User;
use anyhow::Result;

#[derive(Debug, ToSchema)]
pub struct MapDB {
    users: HashMap<String, User>,
    blocks: Mutex<HashMap<String, Vec<u8>>>,
}

impl MapDB {
    pub fn new(users: &[User]) -> Self {
        Self {
            users: users
                .iter()
                .map(|u| (u.username.clone(), u.to_owned()))
                .collect(),
            blocks: Mutex::new(HashMap::new()),
        }
    }
}

impl DB for MapDB {
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.users.get(username).cloned()
    }

    async fn block_exists(&self, hash: &str) -> bool {
        let blocks = self.blocks.lock().unwrap();
        blocks.contains_key(hash)
    }

    async fn store_block(&self, hash: &str, data: Vec<u8>) -> Result<bool, anyhow::Error> {
        let mut blocks = self.blocks.lock().unwrap();

        // Check if the block already exists
        if blocks.contains_key(hash) {
            return Ok(false); // Block already exists, not newly stored
        }

        // Insert the new block with its data
        blocks.insert(hash.to_string(), data);
        Ok(true) // Block was newly stored
    }

    async fn get_block(&self, hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let blocks = self.blocks.lock().unwrap();

        // Check if the block exists and return its data if found
        if let Some(data) = blocks.get(hash) {
            return Ok(Some(data.clone()));
        }

        // Block not found
        Ok(None)
    }
}
