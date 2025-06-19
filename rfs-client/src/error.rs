use thiserror::Error;

/// Errors that can occur when using the RFS client
#[derive(Error, Debug)]
pub enum RfsError {
    /// Error from the underlying OpenAPI client
    #[error("OpenAPI client error: {0}")]
    OpenApiError(String),

    /// Error when making HTTP requests
    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    /// Error when parsing URLs
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// Error when parsing JSON
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// File system error
    #[error("File system error: {0}")]
    FileSystemError(String),

    /// Block management error
    #[error("Block management error: {0}")]
    BlockError(String),

    /// FList management error
    #[error("FList management error: {0}")]
    FListError(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    TimeoutError(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type for RFS client operations
pub type Result<T> = std::result::Result<T, RfsError>;

/// Convert OpenAPI errors to RfsError
pub(crate) fn map_openapi_error<E: std::fmt::Display>(err: E) -> RfsError {
    RfsError::OpenApiError(err.to_string())
}
