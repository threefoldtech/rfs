use anyhow::Result;
use std::path::Path;
use tokio::fs::File;

use crate::upload::{split_file_into_blocks, BLOCK_SIZE};

use crate::server_api;

/// Checks if a file exists in the server splitting it into blocks
pub async fn exists<P: AsRef<Path>>(
    file_path: P,
    server_url: String,
    block_size: Option<usize>,
) -> Result<()> {
    // Use provided block size or default
    let block_size = block_size.unwrap_or(BLOCK_SIZE);
    let file_path = file_path.as_ref();

    info!("Checking file: {}", file_path.display());
    info!("Using block size: {} bytes", block_size);

    // Read the file size
    let file_size = File::open(file_path).await?.metadata().await?.len();

    info!("File size: {} bytes", file_size);
    info!("Splitting file into blocks of {} bytes", block_size);

    // Split file into blocks and calculate hashes
    let (blocks, _) = split_file_into_blocks(file_path, block_size).await?;
    info!("File split into {} blocks", blocks.len());

    // Create futures for all block checks
    let futures = blocks.iter().map(|block_hash| {
        let server_url = server_url.clone();
        let block_hash = block_hash.clone();
        async move {
            let result = server_api::check_block(&server_url, &block_hash).await;
            match result {
                Ok(true) => true, // Block exists
                Ok(false) => {
                    info!("Block with hash {} does not exist on server", block_hash);
                    false
                }
                Err(e) => {
                    info!("Error checking block {}: {}", block_hash, e);
                    false
                }
            }
        }
    });

    // Run all futures concurrently
    let results = futures::future::join_all(futures).await;

    // Process results
    for block_exists in results {
        match block_exists {
            true => {
                info!("File exists on server");
            }
            false => {
                info!("File does not exist on server");
            }
        }
    }

    Ok(())
}

/// Checks if a hash exists in the server
pub async fn exists_by_hash(hash: String, server_url: String) -> Result<()> {
    match server_api::get_blocks_by_hash(&hash, server_url.clone()).await {
        Ok(blocks) if !blocks.is_empty() => {
            info!("Hash exists on server\nHash: {}", hash);
        }
        Ok(_) => {
            info!("Hash does not exist on server");
        }
        Err(_) => {
            info!("Hash does not exist on server");
        }
    }
    Ok(())
}
