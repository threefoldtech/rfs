use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Storage {
    base_dir: PathBuf,
}

impl Storage {
    pub fn new(base_dir: &str) -> Self {
        let base_path = PathBuf::from(base_dir).join("blocks");
        fs::create_dir_all(&base_path).expect("Failed to create storage directory");
        Self {
            base_dir: base_path,
        }
    }

    pub fn save_block(&self, hash: &str, content: &[u8]) -> io::Result<()> {
        let sub_dir = self.base_dir.join(&hash[..4]);
        fs::create_dir_all(&sub_dir)?;

        let block_path = sub_dir.join(hash);
        let mut file = fs::File::create(block_path)?;
        file.write_all(content)
    }

    pub fn get_block(&self, hash: &str) -> io::Result<Option<Vec<u8>>> {
        let block_path = self.base_dir.join(&hash[..4]).join(hash);
        if block_path.exists() {
            Ok(Some(fs::read(block_path)?))
        } else {
            Ok(None)
        }
    }

    pub fn block_exists(&self, hash: &str) -> bool {
        let block_path = self.base_dir.join(&hash[..4]).join(hash);
        block_path.exists()
    }

    pub fn list_blocks(&self) -> io::Result<Vec<String>> {
        let mut block_hashes = Vec::new();

        // Walk through the storage directory
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Each subdirectory represents the first 4 characters of the hash
                for block_entry in fs::read_dir(path)? {
                    let block_entry = block_entry?;
                    let block_path = block_entry.path();
                    if block_path.is_file() {
                        if let Some(file_name) = block_path.file_name() {
                            if let Some(hash) = file_name.to_str() {
                                block_hashes.push(hash.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(block_hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage() {
        let storage = Storage::new("test_storage");

        let hash = "abcd1234";
        let content = b"Hello, world!";

        // Save block
        storage.save_block(hash, content).unwrap();
        assert!(storage.block_exists(hash));

        let hash = "abcd12345";
        let content = b"Hello, world!";

        // Get block
        storage.save_block(hash, content).unwrap();
        let retrieved_content = storage.get_block(hash).unwrap();
        assert_eq!(retrieved_content.unwrap(), content);

        // Clean up
        fs::remove_dir_all("test_storage").unwrap();
    }
}
