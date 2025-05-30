use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::server_api::{self, VerifyBlock};

const PARALLEL_OPERATIONS: usize = 20;
const DEFAULT_PAGE_SIZE: usize = 50;

/// Syncs a file or block between two servers using its hash
pub async fn sync(hash: Option<&str>, source_server: &str, dest_server: &str) -> Result<()> {
    if hash.is_some() {
        return sync_blocks(hash.unwrap(), source_server, dest_server).await;
    }
    sync_all_blocks(source_server, dest_server, Some(DEFAULT_PAGE_SIZE)).await
}

/// Syncs all blocks of a file between two servers
async fn sync_blocks(file_hash: &str, source_server: &str, dest_server: &str) -> Result<()> {
    // Get all blocks for the file from source server
    info!("Getting blocks for file hash: {}", file_hash);
    let blocks = server_api::get_blocks_by_hash(file_hash, source_server.to_string()).await?;

    if blocks.is_empty() {
        return Err(anyhow::anyhow!(
            "No blocks found for file hash: {}",
            file_hash
        ));
    }

    info!("File has {} blocks", blocks.len());

    // Create a client for API requests
    let client = Arc::new(Client::new());

    // Prepare blocks with metadata for verification
    let blocks_with_metadata: Vec<VerifyBlock> = blocks
        .iter()
        .map(|(hash, idx)| VerifyBlock {
            block_hash: hash.clone(),
            file_hash: file_hash.to_string(),
            block_index: *idx,
        })
        .collect();

    // Verify which blocks are missing on the destination server
    let missing_blocks = server_api::verify_blocks_with_server(
        &client,
        dest_server.to_string(),
        blocks_with_metadata,
    )
    .await?;

    if missing_blocks.is_empty() {
        info!("All blocks already exist on destination server");
        return Ok(());
    }

    info!(
        "{} of {} blocks are missing on destination server",
        missing_blocks.len(),
        blocks.len()
    );

    // Create a semaphore to limit concurrent operations
    let semaphore = Arc::new(Semaphore::new(PARALLEL_OPERATIONS));

    // Download missing blocks from source and upload to destination in parallel
    let results = stream::iter(blocks.iter())
        .filter_map(|(block_hash, block_idx)| {
            let is_missing = missing_blocks.iter().any(|hash| hash == block_hash);

            if !is_missing {
                return futures::future::ready(None);
            }

            let block_hash = block_hash.clone();
            let source_server = source_server.to_string();
            let dest_server = dest_server.to_string();
            let file_hash = file_hash.to_string();
            let block_idx = *block_idx;
            let permit = semaphore.clone();
            let client = client.clone();

            futures::future::ready(Some(async move {
                // Acquire a permit from the semaphore
                let _permit = permit
                    .acquire()
                    .await
                    .expect("Failed to acquire semaphore permit");

                info!("Syncing block {} (index: {})", block_hash, block_idx);

                // Download the block from source server
                match server_api::download_block(&block_hash, &source_server).await {
                    Ok(content) => {
                        // Upload the block to destination server
                        server_api::upload_block(
                            client,
                            dest_server,
                            block_hash.clone(),
                            content.to_vec(),
                            file_hash,
                            block_idx,
                        )
                        .await
                        .map_err(|e| (block_hash.clone(), e))
                    }
                    Err(e) => Err((block_hash.clone(), e)),
                }
            }))
        })
        .buffer_unordered(PARALLEL_OPERATIONS)
        .collect::<Vec<_>>()
        .await;

    // Check for any errors in the sync operations
    let mut has_errors = false;
    for result in results {
        if let Err((block_hash, e)) = result {
            has_errors = true;
            error!("Failed to sync block {}: {}", block_hash, e);
        }
    }

    if has_errors {
        Err(anyhow::anyhow!("Some blocks failed to sync"))
    } else {
        info!("All blocks synced successfully");
        Ok(())
    }
}

/// Syncs all blocks between two servers
pub async fn sync_all_blocks(
    source_server: &str,
    dest_server: &str,
    page_size: Option<usize>,
) -> Result<()> {
    info!("Starting full block sync between servers");
    info!("Source server: {}", source_server);
    info!("Destination server: {}", dest_server);

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    let client = Arc::new(Client::new());
    let semaphore = Arc::new(Semaphore::new(PARALLEL_OPERATIONS));

    let mut page = 1;
    let mut total_blocks = 0;
    let mut total_synced = 0;
    let mut total_failed = 0;

    loop {
        // Get a page of blocks from the source server
        info!("Fetching blocks page {} (size: {})", page, page_size);
        let (blocks, total) = match server_api::list_blocks(source_server, page_size, page).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to list blocks from source server: {}", e);
                return Err(anyhow::anyhow!("Failed to list blocks from source server"));
            }
        };

        if blocks.is_empty() {
            info!("No more blocks to sync");
            break;
        }

        total_blocks = total;
        info!(
            "Retrieved {} blocks (page {}/{})",
            blocks.len(),
            page,
            (total_blocks as f64 / page_size as f64).ceil() as usize
        );

        // Process blocks in parallel
        let results = stream::iter(blocks.iter())
            .map(|block_hash| {
                let block_hash = block_hash.clone();
                let source_server = source_server.to_string();
                let dest_server = dest_server.to_string();
                let permit = semaphore.clone();
                let client = client.clone();

                async move {
                    // Acquire a permit from the semaphore
                    let _permit = permit
                        .acquire()
                        .await
                        .expect("Failed to acquire semaphore permit");

                    // Check if block exists on destination server
                    match server_api::check_block(&dest_server, &block_hash).await {
                        Ok(exists) => {
                            if exists {
                                // Block already exists on destination server
                                debug!("Block {} already exists on destination server", block_hash);
                                return Ok(block_hash);
                            }

                            info!("Syncing block {}", block_hash);

                            // Download the block from source server
                            match server_api::download_block(&block_hash, &source_server).await {
                                Ok(content) => {
                                    // Upload the block to destination server
                                    // Note: We don't have file_hash and block_index for this block
                                    // so we use empty string and 0 as placeholders
                                    server_api::upload_block(
                                        client,
                                        dest_server,
                                        block_hash.clone(),
                                        content.to_vec(),
                                        "".to_string(), // file_hash placeholder
                                        0,              // block_index placeholder
                                    )
                                    .await
                                    .map_err(|e| (block_hash.clone(), e))
                                    .map(|_| block_hash)
                                }
                                Err(e) => Err((block_hash.clone(), e)),
                            }
                        }
                        Err(e) => {
                            error!("Failed to check if block {} exists: {}", block_hash, e);
                            Err((block_hash, e))
                        }
                    }
                }
            })
            .buffer_unordered(PARALLEL_OPERATIONS)
            .collect::<Vec<_>>()
            .await;

        // Process results
        for result in results {
            match result {
                Ok(_) => total_synced += 1,
                Err((block_hash, e)) => {
                    total_failed += 1;
                    error!("Failed to sync block {}: {}", block_hash, e);
                }
            }
        }

        info!(
            "Progress: {}/{} blocks synced ({} failed)",
            total_synced, total_blocks, total_failed
        );

        // Move to the next page
        page += 1;
    }

    info!(
        "Block sync completed: {}/{} blocks synced ({} failed)",
        total_synced, total_blocks, total_failed
    );

    if total_failed > 0 {
        Err(anyhow::anyhow!("{} blocks failed to sync", total_failed))
    } else {
        Ok(())
    }
}
