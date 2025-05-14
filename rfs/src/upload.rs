use anyhow::{Context, Result};
use futures::future::join_all;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;

use crate::server_api;

pub const BLOCK_SIZE: usize = 1024 * 1024; // 1MB blocks, same as server
const PARALLEL_UPLOAD: usize = 20; // Number of blocks to upload in parallel

/// Splits the file into blocks and calculates their hashes
pub async fn split_file_into_blocks(
    file_path: &Path,
    block_size: usize,
) -> Result<(Vec<String>, Vec<(String, Vec<u8>)>)> {
    let mut file = File::open(file_path).await.context("Failed to open file")?;
    let mut blocks = Vec::new();
    let mut block_data = Vec::new();

    loop {
        let mut buffer = vec![0; block_size];
        let bytes_read = file.read(&mut buffer).await?;

        if bytes_read == 0 {
            break;
        }

        buffer.truncate(bytes_read);

        // Calculate hash for this block
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash = format!("{:x}", hasher.finalize());

        blocks.push(hash.clone());
        block_data.push((hash, buffer));
    }

    Ok((blocks, block_data))
}

/// Calculates the hash of the entire file by combining the hashes of all blocks
pub fn calculate_file_hash(blocks: &[String]) -> String {
    let mut hasher = Sha256::new();
    for block_hash in blocks {
        hasher.update(block_hash.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Uploads a file to the server, splitting it into blocks and only uploading missing blocks
pub async fn upload<P: AsRef<Path>>(
    file_path: P,
    server_url: String,
    block_size: Option<usize>,
) -> Result<()> {
    let block_size = block_size.unwrap_or(BLOCK_SIZE); // Use provided block size or default
    let file_path = file_path.as_ref();

    info!("Uploading file: {}", file_path.display());
    info!("Using block size: {} bytes", block_size);

    // Create HTTP client
    let client = Client::new();

    // Read the file size
    let file_size = File::open(file_path).await?.metadata().await?.len();

    info!("File size: {} bytes", file_size);
    info!("Splitting file into blocks of {} bytes", block_size);

    // Split file into blocks and calculate hashes
    let (blocks, block_data) = split_file_into_blocks(file_path, block_size).await?;
    info!("File split into {} blocks", blocks.len());

    // Calculate the file hash by combining all block hashes
    let file_hash = calculate_file_hash(&blocks);
    info!("Calculated file hash: {}", file_hash);

    // Prepare blocks with metadata for verification
    let blocks_with_metadata: Vec<server_api::VerifyBlock> = blocks
        .iter()
        .enumerate()
        .map(|(idx, hash)| server_api::VerifyBlock {
            block_hash: hash.clone(),
            file_hash: file_hash.clone(),
            block_index: idx as u64,
        })
        .collect();

    // Verify which blocks are missing on the server
    let missing_blocks =
        server_api::verify_blocks_with_server(&client, server_url.clone(), blocks_with_metadata)
            .await?;
    info!(
        "{} of {} blocks are missing and need to be uploaded",
        missing_blocks.len(),
        block_data.len()
    );

    // Upload missing blocks in parallel
    let client = Arc::new(client);
    let missing_blocks = Arc::new(missing_blocks);

    // Use a semaphore to limit concurrent uploads
    let semaphore = Arc::new(Semaphore::new(PARALLEL_UPLOAD));

    // Create a vector to hold all upload tasks
    let mut upload_tasks = Vec::new();

    for (idx, (hash, data)) in block_data.into_iter().enumerate() {
        if missing_blocks.iter().any(|block| block == &hash) {
            let hash_clone = hash.clone();
            let server_url_clone = server_url.clone();
            let client_clone = Arc::clone(&client);
            let semaphore_clone = Arc::clone(&semaphore);
            let file_hash_clone = file_hash.clone();

            // Create a task for each block upload
            let task: tokio::task::JoinHandle<std::result::Result<(), anyhow::Error>> =
                tokio::spawn(server_api::upload_block(
                    client_clone,
                    server_url_clone,
                    hash_clone,
                    data,
                    file_hash_clone,
                    idx as u64,
                    semaphore_clone,
                ));

            upload_tasks.push(task);
        }
    }

    // Wait for all upload tasks to complete
    let results = join_all(upload_tasks).await;

    // Check for any errors in the upload tasks
    for result in results {
        match result {
            Ok(task_result) => {
                if let Err(e) = task_result {
                    return Err(e);
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Upload task failed: {}", e));
            }
        }
    }

    info!("File upload complete");
    Ok(())
}
