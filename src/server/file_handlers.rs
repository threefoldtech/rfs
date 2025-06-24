use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::server::{
    auth,
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
    components(schemas(File, FileUploadResponse, FileDownloadRequest)),
    tags(
        (name = "files", description = "File management API")
    )
)]
pub struct FileApi;

/// Response for file upload
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileUploadResponse {
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
    extension: axum::extract::Extension<String>,
    body: Bytes,
) -> Result<(StatusCode, ResponseResult), ResponseError> {
    // Convert the request body to a byte vector
    let data = body.to_vec();

    // Create a new File record
    let file = File::new(data.clone());

    // Store the file metadata in the database
    // In a real implementation, we would store this in the files table
    // For now, we'll just log it
    log::info!("Storing file metadata: hash={}", file.file_hash);

    // Get the username from the extension (set by the authorize middleware)
    let username = extension.0;
    if username.is_empty() {
        log::error!("Username is required but not provided");
        return Err(ResponseError::BadRequest("Username is required".to_string()));
    }
    let user_id = auth::get_user_id_from_token(&*state.db, &username).await?;

    // Store each block with a reference to the file
    for (i, chunk) in data
        .chunks(state.config.block_size.unwrap_or(BLOCK_SIZE))
        .enumerate()
    {
        let block_hash = Block::calculate_hash(chunk);

        // TODO: parallel
        // Store each block in the storage with file hash and block index in metadata in DB
        match state
            .db
            .store_block(
                &block_hash,
                chunk.to_vec(),
                &file.file_hash,
                i as u64,
                user_id,
            )
            .await
        {
            Ok(_) => {
                log::debug!("Stored block {}", block_hash);
            }
            Err(err) => {
                log::error!("Failed to store block: {}", err);
                return Err(ResponseError::InternalServerError);
            }
        }
    }

    log::info!(
        "Stored file metadata and blocks for file {}",
        file.file_hash
    );

    // Return success response
    let response = FileUploadResponse {
        file_hash: file.file_hash,
        message: "File is uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, ResponseResult::FileUploaded(response)))
}

/// Request for file download with custom filename
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileDownloadRequest {
    /// The custom filename to use for download
    pub file_name: String,
}

/// Retrieve a file by its hash from path, with optional custom filename in request body.
/// The file will be reconstructed from its blocks.
#[utoipa::path(
    post,
    path = "/api/v1/file/{hash}",
    request_body(content = FileDownloadRequest, description = "Optional custom filename for download", content_type = "application/json"),
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
    request: Option<axum::extract::Json<FileDownloadRequest>>,
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

    // Set content disposition header with the custom filename from request if provided
    // Otherwise use the hash as the filename
    let filename = match request {
        Some(req) => req.0.file_name,
        None => format!("{}.bin", hash), // Default filename using hash
    };

    let headers = [(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename),
    )];

    // Return the file data
    Ok((
        StatusCode::OK,
        headers,
        axum::body::Bytes::from(file.file_content),
    ))
}
