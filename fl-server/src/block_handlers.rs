use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::{
    config::AppState,
    db::DB,
    models::Block,
    response::{ResponseError, ResponseResult},
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(upload_block_handler, get_block_handler, check_block_handler),
    components(schemas(Block)),
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
    path = "/v1/api/block",
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

    // Store the block data in the database
    match state.db.store_block(&hash, data).await {
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
    path = "/v1/api/block/{hash}",
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
    get,
    path = "/v1/api/block/{hash}",
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
