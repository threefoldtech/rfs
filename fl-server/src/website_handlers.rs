use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_macros::debug_handler;
use mime_guess::from_path;
use rfs::fungi::{meta, Reader};
use std::fs;
use std::sync::Arc;
use tempfile::NamedTempFile;
use utoipa::OpenApi;

use crate::{config::AppState, db::DB, response::ResponseError};

#[derive(OpenApi)]
#[openapi(
    paths(serve_website_handler),
    tags(
        (name = "websites", description = "Website serving API")
    )
)]
pub struct WebsiteApi;

/// Resolves a file path within a flist database to get file information
async fn get_file_from_flist(flist_content: &[u8], file_path: &str) -> Result<Vec<meta::Block>> {
    // Create a temporary file
    let temp_file = NamedTempFile::new().context("failed to create temporary file")?;

    // Write flist content to the temporary file
    fs::write(temp_file.path(), flist_content)
        .context("failed to write flist content to temporary file")?;

    // Open the flist file as a database using the existing Reader
    let reader = Reader::new(temp_file.path().to_str().unwrap())
        .await
        .context("failed to open flist as a database")?;

    // Find the root inode
    let root_inode: u64 = reader
        .root_inode()
        .await
        .context("failed to find root inode")?
        .ino;

    // Split the path and traverse
    let mut current_inode = root_inode;
    let path_components: Vec<&str> = file_path.split('/').collect();

    for (i, component) in path_components.iter().enumerate() {
        if component.is_empty() {
            continue;
        }

        // If this is the last component, get file info
        if i == path_components.len() - 1 {
            let file_inode = match reader.lookup(current_inode, component).await {
                Ok(inode) => match inode {
                    Some(inode) => inode.ino,
                    None => {
                        anyhow::bail!("file not found");
                    }
                },
                Err(err) => return Err(anyhow::Error::new(err).context("failed to lookup inode")),
            };

            // Get blocks
            let blocks: Vec<meta::Block> = reader
                .blocks(file_inode)
                .await
                .context("failed to get blocks")?;

            return Ok(blocks);
        }

        // Find the next inode in the path
        current_inode = match reader.lookup(current_inode, component).await {
            Ok(inode) => match inode {
                Some(inode) => inode.ino,
                None => {
                    anyhow::bail!("directory not found");
                }
            },
            Err(err) => return Err(anyhow::Error::new(err).context("failed to lookup inode")),
        };
    }

    anyhow::bail!("file not found")
}

async fn decrypt_block(state: &Arc<AppState>, block: &meta::Block) -> Result<Vec<u8>> {
    let encrypted = match state.db.get_block(&hex::encode(block.id)).await {
        Ok(Some(block_content)) => block_content,
        Ok(None) => {
            anyhow::bail!("Block {:?} not found", block.id);
        }
        Err(err) => {
            anyhow::bail!("Failed to get block {:?}: {}", block.id, err);
        }
    };

    let cipher =
        Aes256Gcm::new_from_slice(&block.key).map_err(|_| anyhow::anyhow!("key is invalid"))?;
    let nonce = Nonce::from_slice(&block.key[..12]);

    let compressed = cipher
        .decrypt(nonce, encrypted.as_slice())
        .map_err(|_| anyhow::anyhow!("encryption error"))?;

    let mut decoder = snap::raw::Decoder::new();
    let plain = decoder.decompress_vec(&compressed)?;

    Ok(plain)
}

#[utoipa::path(
    get,
    path = "/v1/api/website/{website_hash}/{path:.*}",
    responses(
        (status = 200, description = "Website file served successfully"),
        (status = 404, description = "File not found"),
        (status = 500, description = "Internal server error"),
    ),
    params(
        ("website_hash" = String, Path, description = "flist hash of the website directory"),
        ("path" = String, Path, description = "Path to the file within the website directory, defaults to index.html if empty")
    )
)]
#[debug_handler]
pub async fn serve_website_handler(
    State(state): State<Arc<AppState>>,
    Path((website_hash, path)): Path<(String, String)>,
) -> impl IntoResponse {
    // If no path is provided, default to index.html
    let file_path = if path.is_empty() {
        "index.html".to_string()
    } else {
        path
    };

    // Get the flist using the website hash
    let flist = match state.db.get_file_by_hash(&website_hash).await {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(ResponseError::NotFound(format!(
                "Flist with hash '{}' not found",
                website_hash
            )));
        }
        Err(err) => {
            log::error!("Failed to retrieve flist metadata: {}", err);
            return Err(ResponseError::InternalServerError);
        }
    };

    // Resolve the file information from the flist content
    let file_blocks = match get_file_from_flist(&flist.file_content, &file_path).await {
        Ok(blocks) => blocks,
        Err(err) => {
            log::error!(
                "Failed to resolve file '{}' from flist '{}': {}",
                file_path,
                website_hash,
                err
            );
            return Err(ResponseError::NotFound(format!(
                "File {} not found in flist {}",
                file_path, website_hash
            )));
        }
    };

    let mut file_content = Vec::new();
    for block in file_blocks {
        match decrypt_block(&state, &block).await {
            Ok(block_content) => file_content.extend(block_content),
            Err(err) => {
                log::error!(
                    "Failed to decrypt block {:?} for file '{}' in website '{}': {}",
                    block.id,
                    file_path,
                    website_hash,
                    err
                );
                return Err(ResponseError::InternalServerError);
            }
        }
    }

    let mime_type = from_path(&file_path).first_or_octet_stream();

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, mime_type.to_string())],
        file_content,
    )
        .into_response())
}
