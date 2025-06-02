use anyhow::{Context, Result};
use futures::future::join_all;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;

use crate::fungi;
use crate::server_api;
use crate::store;

pub const BLOCK_SIZE: usize = 1024 * 1024; // 1MB blocks, same as server
const PARALLEL_UPLOAD: usize = 20; // Number of blocks to upload in parallel

pub fn calculate_hash(data: &[u8]) -> String {
    let hash = blake2b_simd::Params::new().hash_length(32).hash(data);
    hex::encode(hash.as_bytes())
}

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
        let hash = calculate_hash(&buffer);

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
/// Returns the hash of the uploaded file
pub async fn upload<P: AsRef<Path>>(
    file_path: P,
    server_url: String,
    block_size: Option<usize>,
    token: &str,
) -> Result<String> {
    if token.is_empty() {
        return Err(anyhow::anyhow!("Authentication token is required. Use --token option or set RFS_TOKEN environment variable."));
    }

    let block_size = block_size.unwrap_or(BLOCK_SIZE); // Use provided block size or default
    let file_path = file_path.as_ref();

    info!("Uploading file: {}", file_path.display());
    debug!("Using block size: {} bytes", block_size);

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
            let file_hash_clone = file_hash.clone();
            let token_clone = token.to_string();

            // Acquire a permit from the semaphore
            let _permit = semaphore.acquire().await.unwrap();

            // Create a task for each block upload
            let task: tokio::task::JoinHandle<std::result::Result<(), anyhow::Error>> =
                tokio::spawn(server_api::upload_block(
                    client_clone,
                    server_url_clone,
                    hash_clone,
                    data,
                    file_hash_clone,
                    idx as u64,
                    token_clone,
                ));

            upload_tasks.push(task);
        }
    }

    // Wait for all upload tasks to complete
    let results = join_all(upload_tasks).await;

    // Check for any errors in the upload tasks
    for result in results {
        match result {
            Ok(task_result) => task_result?,
            Err(e) => {
                return Err(anyhow::anyhow!("Upload task failed: {}", e));
            }
        }
    }

    info!("File upload complete");
    Ok(file_hash)
}

/// Uploads a directory to the server, processing all files recursively
pub async fn upload_dir<P: AsRef<Path>>(
    dir_path: P,
    server_url: String,
    block_size: Option<usize>,
    token: &str,
    create_flist: bool,
    flist_output: Option<&str>,
) -> Result<()> {
    if token.is_empty() {
        return Err(anyhow::anyhow!("Authentication token is required. Use --token option or set RFS_TOKEN environment variable."));
    }

    let dir_path = dir_path.as_ref().to_path_buf();

    info!("Uploading directory: {}", dir_path.display());
    debug!(
        "Using block size: {} bytes",
        block_size.unwrap_or(BLOCK_SIZE)
    );

    // Collect all files in the directory recursively
    let mut file_paths = Vec::new();
    collect_files(&dir_path, &mut file_paths).context("Failed to read directory")?;

    info!("Found {} files to upload", file_paths.len());

    if !create_flist {
        // Upload each file
        for file_path in file_paths.clone() {
            upload(&file_path, server_url.clone(), block_size, token).await?;
        }

        info!("Directory upload complete");
        return Ok(());
    }

    // Create and handle flist if requested
    info!("Creating flist for the uploaded directory");

    // Create a temporary flist file if no output path is specified
    let flist_path = match flist_output {
        Some(path) => PathBuf::from(path),
        None => {
            let temp_dir = std::env::temp_dir();
            temp_dir.join(format!(
                "{}.fl",
                dir_path.file_name().unwrap_or_default().to_string_lossy()
            ))
        }
    };

    // Create the flist
    let writer = fungi::Writer::new(&flist_path, true)
        .await
        .context("Failed to create flist file")?;

    // Create a store for the server
    let store = store::parse_router(&[format!(
        "{}://{}?token={}",
        store::server::SCHEME,
        server_url.clone(),
        token
    )])
    .await
    .context("Failed to create store")?;

    // Pack the directory into the flist iteratively to avoid stack overflow
    let result =
        tokio::task::spawn_blocking(move || crate::pack(writer, store, dir_path, false, None))
            .await
            .context("Failed to join spawned task")?;

    result.await.context("Failed to create flist")?;

    info!("Flist created at: {}", flist_path.display());

    // Upload the flist file if it was created
    if flist_path.exists() {
        info!("Uploading flist file");
        let flist_hash = upload(&flist_path, server_url.clone(), block_size, token)
            .await
            .context("Failed to upload flist file")?;

        info!("Flist uploaded successfully. Hash: {}", flist_hash);
    }

    Ok(())
}

fn collect_files(dir_path: &Path, file_paths: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut stack = vec![dir_path.to_path_buf()];

    while let Some(current_path) = stack.pop() {
        for entry in std::fs::read_dir(&current_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                file_paths.push(path);
            } else if path.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(())
}

/// Publishes a website by uploading its directory to the server
pub async fn publish_website<P: AsRef<Path>>(
    dir_path: P,
    server_url: String,
    block_size: Option<usize>,
    token: &str,
) -> Result<()> {
    if token.is_empty() {
        return Err(anyhow::anyhow!("Authentication token is required. Use --token option or set RFS_TOKEN environment variable."));
    }

    let dir_path = dir_path.as_ref().to_path_buf();

    debug!("Uploading directory: {}", dir_path.display());
    debug!(
        "Using block size: {} bytes",
        block_size.unwrap_or(BLOCK_SIZE)
    );

    // Collect all files in the directory recursively
    let mut file_paths = Vec::new();
    collect_files(&dir_path, &mut file_paths).context("Failed to read directory")?;

    debug!("Found {} files to upload", file_paths.len());

    // Create and handle flist if requested
    debug!("Creating flist for the uploaded directory");

    // Create a temporary flist file
    let temp_dir = std::env::temp_dir();
    let flist_path = temp_dir.join(format!(
        "{}.fl",
        dir_path.file_name().unwrap_or_default().to_string_lossy()
    ));

    // Create the flist
    let writer = fungi::Writer::new(&flist_path, true)
        .await
        .context("Failed to create flist file")?;

    // Create a store for the server
    let store = store::parse_router(&[format!(
        "{}://{}?token={}",
        store::server::SCHEME,
        server_url.clone(),
        token
    )])
    .await
    .context("Failed to create store")?;

    // Temporarily disable logs for the upload function
    let original_level = log::max_level();
    log::set_max_level(log::LevelFilter::Off);

    // Pack the directory into the flist iteratively to avoid stack overflow
    let result =
        tokio::task::spawn_blocking(move || crate::pack(writer, store, dir_path, false, None))
            .await
            .context("Failed to join spawned task")?;

    result.await.context("Failed to create flist")?;

    debug!("Flist created at: {}", flist_path.display());

    // Upload the flist file if it was created
    if flist_path.exists() {
        debug!("Uploading flist file");

        let flist_hash = upload(&flist_path, server_url.clone(), block_size, token)
            .await
            .context("Failed to upload flist file")?;

        // Restore the original log level
        log::set_max_level(original_level);

        debug!("Flist uploaded successfully. Hash: {}", flist_hash);

        info!("Website published successfully");
        info!("Website hash: {}", flist_hash);
        info!("Website URL: {}/website/{}/", server_url, flist_hash);
    }

    Ok(())
}

pub async fn get_token_from_server(
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    server_api::signin(&client, server_url, username, password).await
}

/// Track user blocks on the server
/// Returns information about the number of blocks and their total size
pub async fn track_blocks(server_url: &str, token: &str, show_details: bool) -> Result<()> {
    if token.is_empty() {
        return Err(anyhow::anyhow!("Authentication token is required. Use --token option or set RFS_TOKEN environment variable."));
    }

    let user_blocks = server_api::get_user_blocks(server_url, token)
        .await
        .context("Failed to get user blocks")?;

    // Calculate total size
    let total_size: u64 = user_blocks.blocks.iter().map(|(_, size)| size).sum();

    println!("User Blocks Summary:");
    println!("Total blocks: {}", user_blocks.total);
    println!(
        "Total size: {} bytes ({:.2} MB)",
        total_size,
        total_size as f64 / (1024.0 * 1024.0)
    );

    // Print individual blocks if there are any
    if show_details && !user_blocks.blocks.is_empty() {
        println!("\nBlock details:");
        for (hash, size) in &user_blocks.blocks {
            println!("  {} - {} bytes", hash, size);
        }
    }

    Ok(())
}
