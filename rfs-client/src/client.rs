use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;
use bytes::Bytes;

use openapi::{
    apis::{
        authentication_api, block_management_api, flist_management_api,
        file_management_api, system_api, website_serving_api,
        configuration::Configuration,
        Error as OpenApiError,
    },
    models::{
        SignInBody, ListBlocksParams,
        VerifyBlocksRequest, VerifyBlocksResponse, VerifyBlock, FlistBody, UserBlocksResponse, BlockDownloadsResponse,
        BlocksResponse, PreviewResponse, FileInfo, FlistState, ResponseResult, FlistStateResponse, BlockUploadedResponse, FileUploadResponse,
    },
};

use crate::error::{RfsError, Result, map_openapi_error};
use crate::types::{ClientConfig, UploadOptions, DownloadOptions, FlistOptions, WaitOptions};

/// Main client for interacting with the RFS server
#[derive(Clone)]
pub struct RfsClient {
    config: Arc<Configuration>,
    client_config: ClientConfig,
    auth_token: Option<String>,
}

impl RfsClient {
    /// Create a new RFS client with the given configuration
    pub fn new(client_config: ClientConfig) -> Self {
        // Create a custom reqwest client with timeout configuration
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(client_config.timeout_seconds))
            .build()
            .unwrap_or_default();
        
        // Create OpenAPI configuration with our custom client
        let mut config = Configuration::new();
        config.base_path = client_config.base_url.clone();
        config.user_agent = Some(format!("rfs-client/0.1.0"));
        config.client = client;
        
        Self {
            config: Arc::new(config),
            client_config,
            auth_token: None,
        }
    }

    /// Create a new RFS client with default configuration
    pub fn default() -> Self {
        Self::new(ClientConfig::default())
    }

    /// Authenticate with the RFS server
    pub async fn authenticate(&mut self) -> Result<()> {
        if let Some(credentials) = &self.client_config.credentials {
            let sign_in_body = SignInBody {
                username: credentials.username.clone(),
                password: credentials.password.clone(),
            };

            let result = authentication_api::sign_in_handler(&self.config, sign_in_body)
                .await
                .map_err(map_openapi_error)?;

            if let Some(token) = Some(result.access_token) {
                // Create a custom reqwest client with timeout configuration
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(self.client_config.timeout_seconds))
                    .build()
                    .unwrap_or_default();
                
                // Create a new configuration with the auth token and timeout
                let mut new_config = Configuration::new();
                new_config.base_path = self.client_config.base_url.clone();
                new_config.user_agent = Some(format!("rfs-client/0.1.0"));
                new_config.bearer_access_token = Some(token.clone());
                new_config.client = client;
                
                self.config = Arc::new(new_config);
                self.auth_token = Some(token);
                Ok(())
            } else {
                Err(RfsError::AuthError("No token received from server".to_string()))
            }
        } else {
            Err(RfsError::AuthError("No credentials provided".to_string()))
        }
    }

    /// Check if the client is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    /// Get system information
    pub async fn get_system_info(&self) -> Result<String> {
        let result = system_api::health_check_handler(&self.config)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result.msg)
    }

    /// Upload a file to the RFS server
    pub async fn upload_file<P: AsRef<Path>>(&self, file_path: P, options: Option<UploadOptions>) -> Result<String> {
        let file_path = file_path.as_ref();
        let _options = options.unwrap_or_default();
        
        // Check if file exists
        if !file_path.exists() {
            return Err(RfsError::FileSystemError(format!("File not found: {}", file_path.display())));
        }
        
        // Use the OpenAPI client to upload the file
        let result = file_management_api::upload_file_handler(&self.config, file_path.to_path_buf())
            .await
            .map_err(map_openapi_error)?;
        
        // Extract the file hash from the response
        match result {
            FileUploadResponse { file_hash, .. } => {
                Ok(file_hash.clone())
            },
            _ => Err(RfsError::Other("Unexpected response type from file upload".to_string())),
        }
    }

    /// Download a file from the RFS server
    pub async fn download_file<P: AsRef<Path>>(&self, file_id: &str, output_path: P, options: Option<DownloadOptions>) -> Result<()> {
        let output_path = output_path.as_ref();
        let _options = options.unwrap_or_default();
        
        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RfsError::FileSystemError(format!("Failed to create directory: {}", e)))?;
        }
        
        // Create a FileDownloadRequest with the filename from the output path
        let file_name = output_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("downloaded_file")
            .to_string();
            
        let download_request = openapi::models::FileDownloadRequest::new(file_name);
        
        // Download the file
        let response = file_management_api::get_file_handler(&self.config, file_id, download_request)
            .await
            .map_err(map_openapi_error)?;
        
        // Read the response body
        let bytes = response.bytes()
            .await
            .map_err(|e| RfsError::RequestError(e))?;
        
        // Write the file to disk
        std::fs::write(output_path, bytes)
            .map_err(|e| RfsError::FileSystemError(format!("Failed to write file: {}", e)))?;
        
        Ok(())
    }

    /// List blocks with optional filtering
    pub async fn list_blocks(&self, params: Option<ListBlocksParams>) -> Result<Vec<String>> {
        let page = params.as_ref().and_then(|p| p.page).flatten();
        let per_page = params.as_ref().and_then(|p| p.per_page).flatten();
        let result = block_management_api::list_blocks_handler(&self.config, page, per_page)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result.blocks)
    }

    /// Verify blocks
    pub async fn verify_blocks(&self, request: VerifyBlocksRequest) -> Result<VerifyBlocksResponse> {
        let result = block_management_api::verify_blocks_handler(&self.config, request)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Create a new FList from a Docker image
    pub async fn create_flist(&self, image_name: &str, options: Option<FlistOptions>) -> Result<String> {
        // Ensure the client is authenticated
        if !self.is_authenticated() {
            return Err(RfsError::AuthError("Authentication required for creating FLists".to_string()));
        }
        
        // Create FList body with the required fields
        let mut flist = FlistBody::new(image_name.to_string());
        
        // Apply options if provided
        if let Some(opts) = options {
            flist.username = opts.username.map(Some);
            flist.password = opts.password.map(Some);
            flist.auth = opts.auth.map(Some);
            flist.email = opts.email.map(Some);
            flist.server_address = opts.server_address.map(Some);
            flist.identity_token = opts.identity_token.map(Some);
            flist.registry_token = opts.registry_token.map(Some);
        }
        
        // Call the API to create the FList
        let result = flist_management_api::create_flist_handler(&self.config, flist)
            .await
            .map_err(map_openapi_error)?;
        
        // Return the job ID
        Ok(result.id)
    }

    /// Get FList state by job ID
    pub async fn get_flist_state(&self, job_id: &str) -> Result<FlistStateResponse> {
        // Ensure the client is authenticated
        if !self.is_authenticated() {
            return Err(RfsError::AuthError("Authentication required for accessing FList state".to_string()));
        }
        
        // Call the API to get the FList state
        let result = flist_management_api::get_flist_state_handler(&self.config, job_id)
                .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }
    
    /// Wait for an FList to be created
    /// 
    /// This method polls the FList state until it reaches a terminal state (Created or Failed)
    /// or until the timeout is reached.
    pub async fn wait_for_flist_creation(&self, job_id: &str, options: Option<WaitOptions>) -> Result<FlistStateResponse> {
        let options = options.unwrap_or_default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(options.timeout_seconds);
        
        loop {
            // Check if we've exceeded the timeout
            if std::time::Instant::now() > deadline {
                return Err(RfsError::TimeoutError(format!(
                    "Timed out waiting for FList creation after {} seconds", 
                    options.timeout_seconds
                )));
            }
            
            // Get the current state
            let state_result = self.get_flist_state(job_id).await;
            
            match state_result {
                Ok(state) => {
                    // Call progress callback if provided
                    if let Some(ref callback) = options.progress_callback {
                        callback(state.flist_state.as_ref());
                    }
                    
                    // Check if we've reached a terminal state
                    match state.flist_state.as_ref() {
                        FlistState::FlistStateCreated(_) => {
                            // Success! FList was created
                            return Ok(state);
                        },
                        FlistState::FlistStateFailed(error_msg) => {
                            // Failure! FList creation failed
                            return Err(RfsError::FListError(format!("FList creation failed: {}", error_msg)));
                        },
                        _ => {
                            // Still in progress, continue polling
                            tokio::time::sleep(std::time::Duration::from_millis(options.poll_interval_ms)).await;
                        }
                    }
                },
                Err(e) => {
                    // If we get a 404 error, it might be because the FList job is still initializing
                    // Just wait and retry
                    println!("Warning: Error checking FList state: {}", e);
                    println!("Retrying in {} ms...", options.poll_interval_ms);
                    tokio::time::sleep(std::time::Duration::from_millis(options.poll_interval_ms)).await;
                }
            }
        }
    }

    /// Check if a block exists
    pub async fn check_block(&self, hash: &str) -> Result<bool> {
        match block_management_api::check_block_handler(&self.config, hash).await {
            Ok(_) => Ok(true),
            Err(OpenApiError::ResponseError(resp)) if resp.status.as_u16() == 404 => Ok(false),
            Err(e) => Err(map_openapi_error(e)),
        }
    }

    /// Get block download statistics
    pub async fn get_block_downloads(&self, hash: &str) -> Result<BlockDownloadsResponse> {
        let result = block_management_api::get_block_downloads_handler(&self.config, hash)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Download a specific block
    pub async fn get_block(&self, hash: &str) -> Result<Bytes> {
        let response = block_management_api::get_block_handler(&self.config, hash)
            .await
            .map_err(map_openapi_error)?;
        
        let bytes = response.bytes().await
            .map_err(|e| RfsError::RequestError(e))?;
        
        Ok(bytes)
    }

    /// Get blocks by hash (file hash or block hash)
    pub async fn get_blocks_by_hash(&self, hash: &str) -> Result<BlocksResponse> {
        let result = block_management_api::get_blocks_by_hash_handler(&self.config, hash)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Get blocks uploaded by the current user
    pub async fn get_user_blocks(&self, page: Option<i32>, per_page: Option<i32>) -> Result<UserBlocksResponse> {
        let result = block_management_api::get_user_blocks_handler(&self.config, page, per_page)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Upload a single block
    pub async fn upload_block(&self, file_hash: &str, idx: i64, data: Vec<u8>) -> Result<String> {
        // Create a temporary file to hold the block data
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(format!("{}-{}", file_hash, idx));
        
        // Write the data to the temporary file
        std::fs::write(&temp_file_path, &data)
            .map_err(|e| RfsError::FileSystemError(format!("Failed to write temporary block file: {}", e)))?;
        
        // Upload the block
        let result = block_management_api::upload_block_handler(
            &self.config,
            file_hash,
            idx,
            temp_file_path.clone(),
        )
        .await
        .map_err(map_openapi_error)?;
        
        // Clean up the temporary file
        if let Err(e) = std::fs::remove_file(temp_file_path) {
            eprintln!("Warning: Failed to remove temporary block file: {}", e);
        }
        
        // Return the hash from the response
        Ok(result.hash)
    }

    /// List all FLists
    pub async fn list_flists(&self) -> Result<HashMap<String, Vec<FileInfo>>> {
        let result = flist_management_api::list_flists_handler(&self.config)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Preview an FList
    pub async fn preview_flist(&self, flist_path: &str) -> Result<PreviewResponse> {
        let result = flist_management_api::preview_flist_handler(&self.config, flist_path)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Get website content
    pub async fn get_website(&self, website_id: &str, path: &str) -> Result<reqwest::Response> {
        let result = website_serving_api::serve_website_handler(&self.config, website_id, path)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<String> {
        let result = system_api::health_check_handler(&self.config)
            .await
            .map_err(map_openapi_error)?;
        
        Ok(result.msg)
    }


    /// Download an FList file
    /// 
    /// This method downloads an FList from the server and saves it to the specified path.
    pub async fn download_flist<P: AsRef<Path>>(&self, flist_path: &str, output_path: P) -> Result<()> {
        let response = flist_management_api::serve_flists(&self.config, flist_path)
            .await
            .map_err(map_openapi_error)?;
        
        let bytes = response.bytes().await
            .map_err(|e| RfsError::RequestError(e))?;
        
        std::fs::write(output_path, &bytes)
            .map_err(|e| RfsError::FileSystemError(e.to_string()))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = RfsClient::default();
        assert!(!client.is_authenticated());
    }
}
