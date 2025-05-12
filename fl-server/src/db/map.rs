use std::collections::HashMap;
use std::sync::Mutex;
use utoipa::ToSchema;

use super::DB;
use crate::models::{File, User};
use anyhow::Result;

#[derive(Debug, ToSchema)]
pub struct MapDB {
    users: HashMap<String, User>,
    blocks: Mutex<HashMap<String, Vec<u8>>>,
    files: Mutex<HashMap<String, File>>,
    file_blocks: Mutex<HashMap<String, Vec<(String, u64)>>>,
}

impl MapDB {
    pub fn new(users: &[User]) -> Self {
        Self {
            users: users
                .iter()
                .map(|u| (u.username.clone(), u.to_owned()))
                .collect(),
            blocks: Mutex::new(HashMap::new()),
            files: Mutex::new(HashMap::new()),
            file_blocks: Mutex::new(HashMap::new()),
        }
    }
}

impl DB for MapDB {
    async fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.users.get(username).cloned()
    }

    async fn block_exists(&self, file_hash: &str, block_index: u64, block_hash: &str) -> bool {
        let blocks = self.blocks.lock().unwrap();
        blocks.contains_key(block_hash)
    }

    async fn store_block(
        &self,
        block_hash: &str,
        data: Vec<u8>,
        file_hash: &str,
        block_index: u64,
    ) -> Result<bool, anyhow::Error> {
        let mut blocks = self.blocks.lock().unwrap();

        // Check if the block already exists
        if blocks.contains_key(block_hash) {
            return Ok(false); // Block already exists, not newly stored
        }

        // Insert the new block with its data
        blocks.insert(block_hash.to_string(), data);

        // Store file hash and block index in a separate map if provided
        // TODO:
        log::debug!(
            "Block {} is part of file {} at index {}",
            block_hash,
            file_hash,
            block_index
        );

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

    async fn get_file_by_hash(&self, hash: &str) -> Result<Option<File>, anyhow::Error> {
        let files = self.files.lock().unwrap();

        // Check if the file exists and return it if found
        if let Some(file) = files.get(hash) {
            return Ok(Some(file.clone()));
        }

        // File not found
        Ok(None)
    }

    async fn get_file_blocks_ordered(
        &self,
        file_hash: &str,
    ) -> Result<Vec<(String, u64)>, anyhow::Error> {
        let file_blocks = self.file_blocks.lock().unwrap();

        // Retrieve the blocks associated with the file hash
        if let Some(blocks) = file_blocks.get(file_hash) {
            return Ok(blocks.clone());
        }

        // No blocks found for the file
        Ok(Vec::new())
    }
}
