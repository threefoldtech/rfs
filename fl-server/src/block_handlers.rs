use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::{
    auth,
    config::AppState,
    db::DB,
    models::Block,
    response::{ResponseError, ResponseResult},
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(upload_block_handler, get_block_handler, check_block_handler, verify_blocks_handler, get_blocks_by_hash_handler, list_blocks_handler, get_user_blocks_handler),
    components(schemas(Block, VerifyBlocksRequest, VerifyBlocksResponse, BlocksResponse, ListBlocksParams, ListBlocksResponse, UserBlocksResponse)),
    tags(
        (name = "blocks", description = "Block management API")
    )
)]
pub struct BlockApi;

/// Query parameters for uploading a block
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadBlockParams {
    /// File hash associated with the block
    pub file_hash: String,
    /// Block index within the file
    pub idx: u64,
}

/// Upload a block to the server.
/// If the block already exists, the server will return a 200 OK response.
/// If the block is new, the server will return a 201 Created response.
#[utoipa::path(
    post,
    path = "/api/v1/block",
    request_body(content = Vec<u8>, description = "Block data to upload", content_type = "application/octet-stream"),
    params(
        ("file_hash" = String, Query, description = "File hash associated with the block"),
        ("idx" = u64, Query, description = "Block index within the file")
    ),
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
    Query(params): Query<UploadBlockParams>,
    extension: axum::extract::Extension<String>,
    body: Bytes,
) -> Result<(StatusCode, ResponseResult), ResponseError> {
    // Convert the body bytes to Vec<u8>
    let data = body.to_vec();

    // Calculate the hash of the block data
    let hash = Block::calculate_hash(&data);

    // Get the username from the extension (set by the authorize middleware)
    let username = extension.0;
    let user_id = auth::get_user_id_from_token(&*state.db, &username).await?;

    // Store the block data in the database
    match state
        .db
        .store_block(&hash, data, &params.file_hash, params.idx, user_id)
        .await
    {
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

/// Checks a block by its hash.
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
    match state.db.block_exists("", 0, &hash).await {
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
pub struct VerifyBlock {
    /// Block hash to verify
    pub block_hash: String,
    /// File hash associated with the block
    pub file_hash: String,
    /// Block index within the file
    pub block_index: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerifyBlocksRequest {
    /// List of blocks to verify
    pub blocks: Vec<VerifyBlock>,
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
    path = "/api/v1/block/verify",
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

    // Check each block in the request
    for block in request.blocks {
        if !state
            .db
            .block_exists(&block.file_hash, block.block_index, &block.block_hash)
            .await
        {
            missing.push(block.block_hash);
        }
    }

    // Return the list of missing blocks
    Ok((
        StatusCode::OK,
        Json(VerifyBlocksResponse {
            missing, // Include missing blocks in the response
        }),
    ))
}

/// Response for blocks by hash endpoint
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BlocksResponse {
    /// List of blocks with their indices
    pub blocks: Vec<(String, u64)>,
}

/// Retrieve blocks by hash (file hash or block hash).
/// If the hash is a file hash, returns all blocks with their block index related to that file.
/// If the hash is a block hash, returns the block itself.
#[utoipa::path(
    get,
    path = "/api/v1/blocks/{hash}",
    responses(
        (status = 200, description = "Blocks found", body = BlocksResponse),
        (status = 404, description = "Hash not found"),
        (status = 500, description = "Internal server error"),
    ),
    params(
        ("hash" = String, Path, description = "File hash or block hash")
    )
)]
#[debug_handler]
pub async fn get_blocks_by_hash_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> Result<impl IntoResponse, ResponseError> {
    // First, try to get file blocks by hash
    match state.db.get_file_blocks_ordered(&hash).await {
        Ok(blocks) if !blocks.is_empty() => {
            // This is a file hash, return all blocks with their indices
            Ok((StatusCode::OK, Json(BlocksResponse { blocks })))
        }
        Ok(_) | Err(_) => {
            // Not a file hash or error occurred, try as block hash
            match state.db.get_block(&hash).await {
                Ok(Some(_)) => {
                    // This is a block hash, return just this block with index 0
                    Ok((
                        StatusCode::OK,
                        Json(BlocksResponse {
                            blocks: vec![(hash.clone(), 0)],
                        }),
                    ))
                }
                Ok(None) => {
                    // Neither file nor block found
                    Err(ResponseError::NotFound(format!(
                        "No file or block with hash '{}' found",
                        hash
                    )))
                }
                Err(err) => {
                    log::error!("Failed to retrieve block: {}", err);
                    Err(ResponseError::InternalServerError)
                }
            }
        }
    }
}

/// Query parameters for listing blocks
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListBlocksParams {
    /// Page number (1-indexed)
    #[schema(default = 1, minimum = 1)]
    pub page: Option<u32>,
    /// Number of items per page
    #[schema(default = 50, minimum = 1, maximum = 100)]
    pub per_page: Option<u32>,
}

/// Response for listing blocks
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListBlocksResponse {
    /// List of block hashes
    pub blocks: Vec<String>,
    /// Total number of blocks
    pub total: u64,
    /// Current page number
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
}

/// List all block hashes in the server with pagination
#[utoipa::path(
    get,
    path = "/api/v1/blocks",
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-indexed)"),
        ("per_page" = Option<u32>, Query, description = "Number of items per page")
    ),
    responses(
        (status = 200, description = "List of block hashes", body = ListBlocksResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    )
)]
#[debug_handler]
pub async fn list_blocks_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListBlocksParams>,
) -> Result<impl IntoResponse, ResponseError> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(50).min(100);

    match state.db.list_blocks(page, per_page).await {
        Ok((blocks, total)) => {
            let response = ListBlocksResponse {
                blocks,
                total,
                page,
                per_page,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(err) => {
            log::error!("Failed to list blocks: {}", err);
            Err(ResponseError::InternalServerError)
        }
    }
}

/// Response for user blocks endpoint
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserBlocksResponse {
    /// List of blocks with their sizes
    pub blocks: Vec<(String, u64)>,
    /// Total number of blocks
    pub total: u64,
}

/// Retrieve all blocks uploaded by a specific user.
#[utoipa::path(
    get,
    path = "/api/v1/user/blocks",
    responses(
        (status = 200, description = "Blocks found", body = UserBlocksResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[debug_handler]
pub async fn get_user_blocks_handler(
    State(state): State<Arc<AppState>>,
    extension: axum::extract::Extension<String>,
) -> Result<impl IntoResponse, ResponseError> {
    // Get the username from the extension (set by the authorize middleware)
    let username = extension.0;
    let user_id = auth::get_user_id_from_token(&*state.db, &username).await?;

    // Get all blocks related to the user
    match state.db.get_user_blocks(user_id).await {
        Ok(blocks) => {
            let total = blocks.len() as u64;
            let response = UserBlocksResponse { blocks, total };
            Ok((StatusCode::OK, Json(response)))
        }
        Err(err) => {
            log::error!("Failed to retrieve user blocks: {}", err);
            Err(ResponseError::InternalServerError)
        }
    }
}
