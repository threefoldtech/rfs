use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::server_api::{self, VerifyBlock};

const PARALLEL_OPERATIONS: usize = 20; // Number of operations to perform in parallel

/// Syncs a file or block between two servers using its hash
pub async fn sync_by_hash(hash: &str, source_server: &str, dest_server: &str) -> Result<()> {
    info!("Syncing {} with hash: {}", "file", hash);
    info!("Source server: {}", source_server);
    info!("Destination server: {}", dest_server);

    sync_blocks(hash, source_server, dest_server).await
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
