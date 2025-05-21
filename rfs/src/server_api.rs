use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyBlock {
    /// Block hash to verify
    pub block_hash: String,
    /// File hash associated with the block
    pub file_hash: String,
    /// Block index within the file
    pub block_index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyBlocksRequest {
    blocks: Vec<VerifyBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyBlocksResponse {
    missing: Vec<String>,
}

/// Response structure for the blocks endpoint
#[derive(Debug, Serialize, Deserialize)]
struct BlocksResponse {
    blocks: Vec<(String, u64)>,
}

/// Response for listing blocks
#[derive(Debug, Serialize, Deserialize)]
pub struct ListBlocksResponse {
    pub blocks: Vec<String>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

/// Downloads blocks associated with a hash (file hash or block hash)
/// Returns a vector of (block_hash, block_index) pairs
pub async fn get_blocks_by_hash(hash: &str, server_url: String) -> Result<Vec<(String, u64)>> {
    info!("Getting blocks for hash: {}", hash);

    // Create HTTP client
    let client = Client::new();

    // Construct the blocks URL
    let blocks_url = format!("{}/api/v1/blocks/{}", server_url, hash);

    info!("Requesting blocks from: {}", blocks_url);

    // Send GET request to get the blocks
    let response = client
        .get(&blocks_url)
        .send()
        .await
        .context("Failed to get blocks from server")?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned error: {} - {}",
            response.status(),
            response.text().await?
        ));
    }

    // Parse the response
    let blocks_response: BlocksResponse = response
        .json()
        .await
        .context("Failed to parse blocks response")?;

    info!("Retrieved {} blocks", blocks_response.blocks.len());

    Ok(blocks_response.blocks)
}

pub async fn download_block(block_hash: &str, server_url: &str) -> Result<bytes::Bytes> {
    let block_url = format!("{}/api/v1/block/{}", server_url, block_hash);

    // Create HTTP client
    let client = Client::new();

    // Send GET request to download the block
    let response = client
        .get(&block_url)
        .send()
        .await
        .context(format!("Failed to download block {}", block_hash))?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned error for block {}: {} - {}",
            block_hash,
            response.status(),
            response.text().await?
        ));
    }

    // Get the block content
    let block_content = response
        .bytes()
        .await
        .context("Failed to read block content")?;
    info!(
        "Downloaded block {} ({} bytes)",
        block_hash,
        block_content.len()
    );

    Ok(block_content)
}

/// Verifies which blocks are missing on the server
pub async fn verify_blocks_with_server(
    client: &Client,
    server_url: String,
    blocks: Vec<VerifyBlock>,
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
pub async fn upload_block(
    client: Arc<Client>,
    server_url: String,
    hash: String,
    data: Vec<u8>,
    file_hash: String,
    idx: u64,
) -> Result<()> {
    let upload_block_url = format!("{}/api/v1/block", server_url);

    info!("Uploading block: {}", hash);

    // Send the data directly as bytes with query parameters
    let response = client
        .post(&upload_block_url)
        .header("Content-Type", "application/octet-stream")
        .query(&[("file_hash", &file_hash), ("idx", &idx.to_string())])
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

    if response.status() == 200 {
        info!("Block {} already exists on server", hash);
    }
    if response.status() == 201 {
        info!("Successfully uploaded block: {}", hash);
    }

    Ok(())
}

/// Checks if a block exists on the server by its hash.
pub async fn check_block(server_url: &str, hash: &str) -> Result<bool> {
    let url = format!("{}/api/v1/block/{}", server_url, hash);

    let client = Client::new();
    let response = client
        .head(&url)
        .send()
        .await
        .context("Failed to send request to check block")?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(true),         // Block exists
        reqwest::StatusCode::NOT_FOUND => Ok(false), // Block does not exist
        _ => Err(anyhow::anyhow!(
            "Unexpected response from server: {}",
            response.status()
        )),
    }
}

/// Lists blocks available on the server with pagination.
/// Returns a vector of (block_hash, block_index) pairs.
pub async fn list_blocks(
    server_url: &str,
    page_size: usize,
    page: usize,
) -> Result<(Vec<String>, u64)> {
    let blocks_url = format!(
        "{}/api/v1/blocks?page={}&page_size={}",
        server_url, page, page_size
    );

    // Create HTTP client
    let client = Client::new();

    // Send GET request to get blocks for the current page
    let response = client
        .get(&blocks_url)
        .send()
        .await
        .context("Failed to list blocks from server")?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned error: {} - {}",
            response.status(),
            response.text().await?
        ));
    }

    // Parse the response
    let blocks_response: ListBlocksResponse = response
        .json()
        .await
        .context("Failed to parse blocks response")?;

    Ok((blocks_response.blocks, blocks_response.total))
}
