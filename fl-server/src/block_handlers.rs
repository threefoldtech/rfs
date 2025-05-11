use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::{
    config::AppState,
    db::DB,
    models::Block,
    response::{ResponseError, ResponseResult},
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(upload_block_handler, get_block_handler, check_block_handler, verify_blocks_handler),
    components(schemas(Block, VerifyBlocksRequest, VerifyBlocksResponse)),
    tags(
        (name = "blocks", description = "Block management API")
    )
)]
pub struct BlockApi;

/// Upload a block to the server.
/// If the block already exists, the server will return a 200 OK response.
/// If the block is new, the server will return a 201 Created response.
#[utoipa::path(
    post,
    path = "/api/v1/block",
    request_body(content = Vec<u8>, description = "Block data to upload", content_type = "application/octet-stream"),
    responses(
        (status = 200, description = "Block already exists", body = String),
        (status = 201, description = "Block created successfully", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    )
)]
#[debug_handler]
pub async fn upload_block_handler(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<(StatusCode, ResponseResult), ResponseError> {
    // Convert the request body to a byte vector
    let data = body.to_vec();

    // Calculate the hash of the block data
    let hash = Block::calculate_hash(&data);

    // Store the block data in the database (not associated with any file)
    match state.db.store_block(&hash, data, None, None).await {
        Ok(is_new) => {
            if is_new {
                // Block is new, return 201 Created
                Ok((StatusCode::CREATED, ResponseResult::BlockUploaded(hash)))
            } else {
                // Block already exists, return 200 OK
                Ok((StatusCode::OK, ResponseResult::BlockUploaded(hash)))
            }
        }
        Err(err) => {
            log::error!("Failed to store block: {}", err);
            Err(ResponseError::InternalServerError)
        }
    }
}

/// Retrieve a block by its hash.
#[utoipa::path(
    get,
    path = "/api/v1/block/{hash}",
    responses(
        (status = 200, description = "Block found", content_type = "application/octet-stream"),
        (status = 404, description = "Block not found"),
        (status = 500, description = "Internal server error"),
    ),
    params(
        ("hash" = String, Path, description = "Block hash")
    )
)]
#[debug_handler]
pub async fn get_block_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ResponseError> {
    // Retrieve the block from the database
    match state.db.get_block(&hash).await {
        Ok(Some(data)) => {
            // Block found, return its data
            Ok((StatusCode::OK, axum::body::Bytes::from(data)))
        }
        Ok(None) => {
            // Block not found
            Err(ResponseError::NotFound(format!(
                "Block with hash '{}' not found",
                hash
            )))
        }
        Err(err) => {
            log::error!("Failed to retrieve block: {}", err);
            Err(ResponseError::InternalServerError)
        }
    }
}

/// Retrieve a block by its hash.
#[utoipa::path(
    head,
    path = "/api/v1/block/{hash}",
    responses(
        (status = 200, description = "Block found"),
        (status = 404, description = "Block not found"),
    ),
    params(
        ("hash" = String, Path, description = "Block hash")
    )
)]
#[debug_handler]
pub async fn check_block_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ResponseError> {
    // Retrieve the block from the database
    match state.db.block_exists(&hash).await {
        true => {
            // Block found
            Ok(StatusCode::OK)
        }
        false => {
            log::error!("Block with hash '{}' doesn't exist", hash);
            Err(ResponseError::NotFound(format!(
                "Block with hash '{}' not found",
                hash
            )))
        }
    }
}

/// Request to verify if multiple blocks exist on the server
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyBlocksRequest {
    /// List of block hashes to verify
    pub blocks: Vec<String>,
}

/// Response with list of missing blocks
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyBlocksResponse {
    /// List of block hashes that are missing on the server
    pub missing: Vec<String>,
}

/// Verify if multiple blocks exist on the server.
/// Returns a list of missing blocks.
#[utoipa::path(
    post,
    path = "/api/v1/files/verify",
    request_body(content = VerifyBlocksRequest, description = "List of block hashes to verify", content_type = "application/json"),
    responses(
        (status = 200, description = "Verification completed", body = VerifyBlocksResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    )
)]
#[debug_handler]
pub async fn verify_blocks_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<VerifyBlocksRequest>,
) -> Result<impl IntoResponse, ResponseError> {
    let mut missing = Vec::new();

    // Check each hash in the request
    for block in request.blocks {
        if !state.db.block_exists(&block).await {
            missing.push(block);
        }
    }

    // Return the list of missing blocks
    Ok((StatusCode::OK, Json(VerifyBlocksResponse { missing })))
}
