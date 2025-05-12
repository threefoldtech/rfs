use anyhow::{Context, Result};
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;

const BLOCK_SIZE: usize = 1024 * 1024; // 1MB blocks, same as server
const PARALLEL_UPLOAD: usize = 20; // Number of blocks to upload in parallel

#[derive(Debug, Serialize, Deserialize)]
struct VerifyBlocksRequest {
    blocks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyBlocksResponse {
    missing: Vec<String>,
}

/// Verifies which blocks are missing on the server
async fn verify_blocks_with_server(
    client: &Client,
    server_url: String,
    blocks: Vec<String>,
) -> Result<Vec<String>> {
    let verify_url = format!("{}/api/v1/block/verify", server_url);
    let verify_request = VerifyBlocksRequest { blocks };

    info!("Verifying blocks with server: {}", verify_url);

    let response = client
        .post(&verify_url)
        .json(&verify_request)
        .send()
        .await
        .context("Failed to verify blocks with server")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned error: {} - {}",
            response.status(),
            response.text().await?
        ));
    }

    let verify_response: VerifyBlocksResponse = response
        .json()
        .await
        .context("Failed to parse server response")?;

    Ok(verify_response.missing)
}

/// Uploads a single block to the server
async fn upload_block(
    client: Arc<Client>,
    server_url: String,
    hash: String,
    data: Vec<u8>,
    semaphore: Arc<Semaphore>,
) -> Result<()> {
    let upload_block_url = format!("{}/api/v1/block", server_url);

    // Acquire a permit from the semaphore
    let _permit = semaphore.acquire().await.unwrap();

    info!("Uploading block: {}", hash);

    let response = client
        .post(&upload_block_url)
        .body(data)
        .send()
        .await
        .context("Failed to upload block")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to upload block {}: {} - {}",
            hash,
            response.status(),
            response.text().await?
        ));
    }

    info!("Successfully uploaded block: {}", hash);
    Ok(())
}

/// Splits the file into blocks and calculates their hashes
async fn split_file_into_blocks(
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

/// Uploads a file to the server, splitting it into blocks and only uploading missing blocks
pub async fn upload<P: AsRef<Path>>(
    file_path: P,
    server_url: String,
    block_size: Option<usize>,
) -> Result<()> {
    let block_size = block_size.unwrap_or(BLOCK_SIZE); // Use provided block size or default
    let file_path = file_path.as_ref();
    let file_name = file_path
        .file_name()
        .context("Invalid file path")?
        .to_string_lossy();

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

    // Verify which blocks are missing on the server
    let missing_blocks = verify_blocks_with_server(&client, server_url.clone(), blocks).await?;
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

    for (hash, data) in block_data {
        if Arc::clone(&missing_blocks).contains(&hash) {
            let hash_clone = hash.clone();
            let server_url_clone = server_url.clone();
            let client_clone = Arc::clone(&client);
            let semaphore_clone = Arc::clone(&semaphore);

            // Create a task for each block upload
            let task = tokio::spawn(upload_block(
                client_clone,
                server_url_clone,
                hash_clone,
                data,
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

    info!("File upload complete: {}", file_name);
    Ok(())
}
