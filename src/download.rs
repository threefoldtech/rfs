use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

use crate::server_api;
use crate::{cache, fungi, store};

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
                server_api::download_block(&block_hash, &server_url)
                    .await
                    .map(|content| (i, content))
                    .map_err(|e| (i, e))
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

/// Downloads a directory by processing all files listed in its flist using the flist hash
pub async fn download_dir<P: AsRef<Path>>(
    flist_hash: &str,
    output_dir: P,
    server_url: String,
) -> Result<()> {
    let output_dir = output_dir.as_ref();

    info!("Downloading directory from flist with hash: {}", flist_hash);
    info!("Saving files to: {}", output_dir.display());

    // Download the flist file using its hash
    let temp_path = std::env::temp_dir().join(format!("{}.fl", flist_hash));
    download(flist_hash, &temp_path, server_url.clone()).await?;

    let meta = fungi::Reader::new(temp_path)
        .await
        .context("failed to initialize metadata database")?;

    let router = store::get_router(&meta).await?;
    let cache = cache::Cache::new("/tmp/cache", router);
    crate::unpack(&meta, &cache, output_dir, false).await?;

    info!("Directory download complete");
    Ok(())
}

/// Track blocks uploaded by the user and their download counts
/// If hash is provided, only track that specific block
/// Otherwise, track all user blocks
pub async fn track_blocks(
    server_url: &str,
    token: &str,
    hash: Option<&str>,
    details: bool,
) -> Result<()> {
    if let Some(block_hash) = hash {
        match server_api::get_block_downloads(server_url, block_hash).await {
            Ok(downloads) => {
                println!(
                    "{:<64} {:<10} {:<10}",
                    "BLOCK HASH", "DOWNLOADS", "SIZE (B)"
                );
                println!("{}", "-".repeat(85));
                println!(
                    "{:<64} {:<10} {:<10}",
                    downloads.block_hash, downloads.downloads_count, downloads.block_size
                );
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to get download count for block {}: {}",
                    block_hash,
                    err
                ));
            }
        }

        return Ok(());
    }

    // Track all user blocks
    let mut all_user_blocks = Vec::new();

    let first_page = server_api::get_user_blocks(server_url, token, Some(1), Some(50))
        .await
        .context("Failed to get user blocks")?;

    let total_pages = (first_page.total as f64 / 50.0).ceil() as u32;

    let mut tasks = Vec::new();
    for page in 1..=total_pages {
        let server_url = server_url.to_string();
        let token = token.to_string();
        tasks.push(tokio::spawn(async move {
            server_api::get_user_blocks(&server_url, &token, Some(page), Some(50)).await
        }));
    }

    for task in tasks {
        match task.await {
            Ok(Ok(blocks_per_page)) => {
                all_user_blocks.extend(blocks_per_page.blocks);
            }
            Ok(Err(err)) => {
                return Err(anyhow::anyhow!("Failed to get user blocks: {}", err));
            }
            Err(err) => {
                return Err(anyhow::anyhow!("Task failed: {}", err));
            }
        }
    }

    println!(
        "User has {} blocks out of {} total blocks on the server",
        all_user_blocks.len(),
        first_page.all_blocks
    );

    let block_hashes: Vec<String> = all_user_blocks
        .into_iter()
        .map(|(block_hash, _)| block_hash)
        .collect();
    print_block_downloads(server_url, block_hashes, details).await?;

    Ok(())
}

pub async fn print_block_downloads(
    server_url: &str,
    blocks: Vec<String>,
    details: bool,
) -> Result<()> {
    // Collect all block details first
    let mut block_details = Vec::new();
    let mut total_downloads = 0;
    let mut bandwidth = 0;

    for block_hash in blocks {
        match server_api::get_block_downloads(server_url, &block_hash).await {
            Ok(downloads) => {
                total_downloads += downloads.downloads_count;
                bandwidth += downloads.block_size * downloads.downloads_count;
                block_details.push(downloads);
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to get download count for block {}: {}",
                    block_hash,
                    err
                ));
            }
        }
    }

    // Print totals first
    println!("{}", "-".repeat(85));
    println!("TOTAL DOWNLOADS: {}", total_downloads);
    println!("BANDWIDTH: {} bytes", bandwidth);

    if details {
        println!("{}", "-".repeat(85));

        println!(
            "\n{:<64} {:<10} {:<10}",
            "BLOCK HASH", "DOWNLOADS", "SIZE (B)"
        );
        println!("{}", "-".repeat(85));

        for block in block_details {
            println!(
                "{:<64} {:<10} {:<10}",
                block.block_hash, block.downloads_count, block.block_size
            );
        }
    }

    Ok(())
}

pub async fn track_website(server_url: &str, flist_hash: &str, details: bool) -> Result<()> {
    // Temporarily disable logs for the upload function
    let original_level = log::max_level();
    log::set_max_level(log::LevelFilter::Off);

    let flist_blocks = server_api::get_blocks_by_hash(flist_hash, server_url.to_owned()).await?;

    if flist_blocks.is_empty() {
        return Err(anyhow::anyhow!("No blocks found for hash: {}", flist_hash));
    }

    // Download the flist file using its hash
    let temp_path = std::env::temp_dir().join(format!("{}.fl", flist_hash));
    download(flist_hash, &temp_path, server_url.to_owned()).await?;

    let meta = fungi::Reader::new(temp_path)
        .await
        .context("failed to initialize metadata database")?;

    let router = store::get_router(&meta).await?;
    let cache_dir = std::env::temp_dir().join("cache_blocks");
    let cache = cache::Cache::new(cache_dir.clone(), router);
    let temp_output_dir = std::env::temp_dir().join("output_dir");
    tokio::fs::create_dir_all(&temp_output_dir)
        .await
        .context("Failed to create temporary output directory")?;
    crate::unpack(&meta, &cache, &temp_output_dir, false).await?;

    // Restore the original log level
    log::set_max_level(original_level);

    let mut website_blocks = list_files_in_dir(cache_dir.clone())
        .await
        .context("Failed to list files in /tmp/cache directory")?;

    website_blocks.extend(flist_blocks.into_iter().map(|(block_hash, _)| block_hash));

    println!("Website has {} blocks on the server", website_blocks.len());
    print_block_downloads(&server_url, website_blocks, details).await?;

    // Delete the temporary directory
    tokio::fs::remove_dir_all(&temp_output_dir)
        .await
        .context("Failed to delete temporary output directory")?;
    tokio::fs::remove_dir_all(&cache_dir)
        .await
        .context("Failed to delete temporary cache directory")?;

    Ok(())
}

pub async fn list_files_in_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<String>> {
    let dir = dir.as_ref();
    let mut file_names = Vec::new();

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .context(format!("Failed to read directory: {}", dir.display()))?;

    while let Some(entry) = entries.next_entry().await.context("Failed to read entry")? {
        let path = entry.path();
        if path.is_dir() {
            let sub_dir_files = Box::pin(list_files_in_dir(path)).await?;
            file_names.extend(sub_dir_files);
            continue;
        }
        if let Ok(file_name) = entry.file_name().into_string() {
            file_names.push(file_name);
        }
    }
    Ok(file_names)
}
