// Re-export common types from OpenAPI client for convenience
pub use openapi::models::{
    Block, BlockDownloadsResponse, BlocksResponse, File, FileInfo, 
    FileUploadResponse, FlistBody, FlistState, Job, ListBlocksResponse,
    PreviewResponse, ResponseResult, SignInResponse, VerifyBlocksResponse,
};

/// Authentication credentials for the RFS server
#[derive(Clone, Debug)]
pub struct Credentials {
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
}

/// Configuration for the RFS client
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// Base URL of the RFS server
    pub base_url: String,
    /// Optional authentication credentials
    pub credentials: Option<Credentials>,
    /// Timeout for API requests in seconds
    pub timeout_seconds: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            credentials: None,
            timeout_seconds: 30,
        }
    }
}

/// Upload options for file uploads
#[derive(Clone, Debug, Default)]
pub struct UploadOptions {
    /// Chunk size for uploading large files
    pub chunk_size: Option<usize>,
    /// Whether to verify blocks after upload
    pub verify: bool,
}

/// Download options for file downloads
#[derive(Clone, Debug, Default)]
pub struct DownloadOptions {
    /// Whether to verify blocks during download
    pub verify: bool,
}

/// Options for creating FLists
#[derive(Clone, Debug, Default)]
pub struct FlistOptions {
    /// Optional username for registry authentication
    pub username: Option<String>,
    /// Optional password for registry authentication
    pub password: Option<String>,
    /// Optional auth token for registry authentication
    pub auth: Option<String>,
    /// Optional email for registry authentication
    pub email: Option<String>,
    /// Optional server address for registry
    pub server_address: Option<String>,
    /// Optional identity token for registry authentication
    pub identity_token: Option<String>,
    /// Optional registry token for registry authentication
    pub registry_token: Option<String>,
}

/// Options for waiting operations
pub struct WaitOptions {
    /// Maximum time to wait in seconds
    pub timeout_seconds: u64,
    
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    
    /// Optional progress callback
    pub progress_callback: Option<Box<dyn Fn(&FlistState) + Send + Sync>>,
}

// Manual implementation of Debug for WaitOptions
impl std::fmt::Debug for WaitOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitOptions")
            .field("timeout_seconds", &self.timeout_seconds)
            .field("poll_interval_ms", &self.poll_interval_ms)
            .field("progress_callback", &if self.progress_callback.is_some() { "Some(...)" } else { "None" })
            .finish()
    }
}

// Manual implementation of Clone for WaitOptions
impl Clone for WaitOptions {
    fn clone(&self) -> Self {
        Self {
            timeout_seconds: self.timeout_seconds,
            poll_interval_ms: self.poll_interval_ms,
            progress_callback: None, // We can't clone the callback function
        }
    }
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            timeout_seconds: 300,  // 5 minutes default timeout
            poll_interval_ms: 1000, // 1 second default polling interval
            progress_callback: None,
        }
    }
}
