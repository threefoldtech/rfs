use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::server_api;

const PARALLEL_DOWNLOAD: usize = 20; // Number of blocks to download in parallel

/// Downloads all blocks for a file or a single block and assembles them
pub async fn download<P: AsRef<Path>>(hash: &str, file_name: P, server_url: String) -> Result<()> {
    let file_name = file_name.as_ref();

    info!("Downloading blocks for hash: {}", hash);
    info!("Saving to: {}", file_name.display());

    let blocks = server_api::get_blocks_by_hash(hash, server_url.clone()).await?;

    if blocks.is_empty() {
        return Err(anyhow::anyhow!("No blocks found for hash: {}", hash));
    }

    // Store the number of blocks
    let blocks_count = blocks.len();

    // Create the file
    let mut file = File::create(file_name)
        .await
        .context("Failed to create output file")?;

    // Create a semaphore to limit concurrent downloads
    let semaphore = std::sync::Arc::new(Semaphore::new(PARALLEL_DOWNLOAD));

    // Download blocks in parallel
    info!(
        "Starting parallel download of {} blocks with concurrency {}",
        blocks_count, PARALLEL_DOWNLOAD
    );

    // Create a vector to store downloaded blocks in order
    let mut downloaded_blocks = vec![None; blocks_count];

    // Process blocks in parallel with limited concurrency
    let results = stream::iter(blocks.into_iter().enumerate())
        .map(|(i, (block_hash, block_index))| {
            let server_url = server_url.clone();
            let permit = semaphore.clone();

            async move {
                // Acquire a permit from the semaphore
                let _permit = permit
                    .acquire()
                    .await
                    .expect("Failed to acquire semaphore permit");

                info!("Downloading block {} (index: {})", block_hash, block_index);

                // Download the block
                let result = server_api::download_block(&block_hash, &server_url)
                    .await
                    .map(|content| (i, content))
                    .map_err(|e| (i, e));

                result
            }
        })
        .buffer_unordered(PARALLEL_DOWNLOAD)
        .collect::<Vec<_>>()
        .await;

    // Process results and write blocks to file in correct order
    for result in results {
        match result {
            Ok((index, content)) => {
                downloaded_blocks[index] = Some(content);
            }
            Err((index, e)) => {
                return Err(anyhow::anyhow!(
                    "Failed to download block at index {}: {}",
                    index,
                    e
                ));
            }
        }
    }

    // Write blocks to file in order
    for (i, block_opt) in downloaded_blocks.into_iter().enumerate() {
        if let Some(block_content) = block_opt {
            file.write_all(&block_content)
                .await
                .context(format!("Failed to write block at index {} to file", i))?;
        } else {
            return Err(anyhow::anyhow!("Missing block at index {}", i));
        }
    }

    info!("File downloaded successfully to {:?}", file_name);

    Ok(())
}
