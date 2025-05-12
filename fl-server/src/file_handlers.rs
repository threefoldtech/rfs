use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse};
use axum_macros::debug_handler;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    config::AppState,
    db::DB,
    models::{Block, File},
    response::{ResponseError, ResponseResult},
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

const BLOCK_SIZE: usize = 1024 * 1024; // 1MB

#[derive(OpenApi)]
#[openapi(
    paths(upload_file_handler, get_file_handler),
    components(schemas(File, FileUploadResponse)),
    tags(
        (name = "files", description = "File management API")
    )
)]
pub struct FileApi;

/// Response for file upload
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileUploadResponse {
    /// The file ID
    pub id: String,
    /// The file hash
    pub file_hash: String,
    /// Message indicating success
    pub message: String,
}

/// Upload a file to the server.
/// The file will be split into blocks and stored in the database.
#[utoipa::path(
    post,
    path = "/api/v1/file",
    request_body(content = Vec<u8>, description = "File data to upload", content_type = "application/octet-stream"),
    responses(
        (status = 201, description = "File uploaded successfully", body = FileUploadResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    )
)]
#[debug_handler]
pub async fn upload_file_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Result<(StatusCode, ResponseResult), ResponseError> {
    // Get the file name from query parameters
    let file_name = params.get("filename").ok_or_else(|| {
        ResponseError::BadRequest("Missing 'filename' query parameter".to_string())
    })?;

    // Convert the request body to a byte vector
    let data = body.to_vec();

    // Calculate the hash of the entire file
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let file_hash_str = format!("{:x}", hasher.finalize());

    // Create a new File record
    let file = File::new(file_name.clone(), file_hash_str.clone());

    // Store the file metadata in the database
    // In a real implementation, we would store this in the files table
    // For now, we'll just log it
    log::info!(
        "Storing file metadata: id={}, name={}, hash={}",
        file.id,
        file.file_name,
        file.file_hash
    );

    // Store each block with a reference to the file
    for (i, chunk) in data
        .chunks(state.config.block_size.unwrap_or(BLOCK_SIZE))
        .enumerate()
    {
        let block_hash = Block::calculate_hash(chunk);

        // Store each block in the database with file hash and block index
        match state
            .db
            .store_block(
                &block_hash,
                chunk.to_vec(),
                Some(file_hash_str.clone()),
                Some(i as u64),
            )
            .await
        {
            Ok(_) => {
                log::debug!("Stored block {} for file {}", block_hash, file_name);
            }
            Err(err) => {
                log::error!("Failed to store block: {}", err);
                return Err(ResponseError::InternalServerError);
            }
        }
    }

    log::info!("Stored file metadata for {}", file_name);

    // Return success response
    let response = FileUploadResponse {
        id: file.id.to_string(),
        file_hash: file.file_hash,
        message: format!("File '{}' uploaded successfully", file_name),
    };

    Ok((StatusCode::CREATED, ResponseResult::FileUploaded(response)))
}

/// Retrieve a file by its hash.
/// The file will be reconstructed from its blocks.
#[utoipa::path(
    get,
    path = "/api/v1/file/{hash}",
    responses(
        (status = 200, description = "File found", content_type = "application/octet-stream"),
        (status = 404, description = "File not found"),
        (status = 500, description = "Internal server error"),
    ),
    params(
        ("hash" = String, Path, description = "File hash")
    )
)]
#[debug_handler]
pub async fn get_file_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ResponseError> {
    // Get the file metadata using the hash
    let file = match state.db.get_file_by_hash(&hash).await {
        Ok(Some(file)) => file,
        Ok(None) => {
            return Err(ResponseError::NotFound(format!(
                "File with hash '{}' not found",
                hash
            )));
        }
        Err(err) => {
            log::error!("Failed to retrieve file metadata: {}", err);
            return Err(ResponseError::InternalServerError);
        }
    };

    // Get all blocks associated with the file
    let blocks = match state.db.get_file_blocks(&hash).await {
        Ok(blocks) => blocks,
        Err(err) => {
            log::error!("Failed to retrieve file blocks: {}", err);
            return Err(ResponseError::InternalServerError);
        }
    };

    // Reconstruct the file from its blocks
    let mut file_data = Vec::new();

    // Sort blocks by index to ensure correct order
    let mut sorted_blocks = blocks;
    sorted_blocks.sort_by_key(|(_, index)| *index);

    // Retrieve and concatenate each block's data
    for (block_hash, _) in sorted_blocks {
        match state.db.get_block(&block_hash).await {
            Ok(Some(data)) => {
                file_data.extend_from_slice(&data);
            }
            Ok(None) => {
                log::error!("Block with hash '{}' not found", block_hash);
                return Err(ResponseError::InternalServerError);
            }
            Err(err) => {
                log::error!("Failed to retrieve block data: {}", err);
                return Err(ResponseError::InternalServerError);
            }
        }
    }

    // Set content disposition header to suggest filename for download
    let headers = [(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", file.file_name),
    )];

    // Return the file data
    Ok((StatusCode::OK, headers, axum::body::Bytes::from(file_data)))
}
