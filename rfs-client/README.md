# RFS Client

A Rust client library for interacting with the Remote File System (RFS) server.

## Overview

This client library provides a user-friendly wrapper around the OpenAPI-generated client code. It offers high-level abstractions for common operations such as:

- Authentication and session management
- File uploads and downloads with progress tracking
- Block-level operations and verification
- FList creation, monitoring, and management
- Timeout configuration and error handling

## Structure

The library is organized as follows:

- `client.rs`: Main client implementation with methods for interacting with the RFS server
- `error.rs`: Error types and handling
- `types.rs`: Type definitions and utilities

## Quick Start

```rust
use rfs_client::RfsClient;
use rfs_client::types::{ClientConfig, Credentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with custom configuration
    let config = ClientConfig {
        base_url: "http://localhost:8080".to_string(),
        credentials: Some(Credentials {
            username: "user".to_string(),
            password: "password".to_string(),
        }),
        timeout_seconds: 60,
    };
    
    let mut client = RfsClient::new(config);
    
    // Authenticate
    client.authenticate().await?;
    println!("Authentication successful");
    
    // Upload a file
    let file_path = "/path/to/file.txt";
    let file_hash = client.upload_file(file_path, None).await?;
    println!("File uploaded with hash: {}", file_hash);
    
    // Download the file
    let output_path = "/path/to/output.txt";
    client.download_file(&file_hash, output_path, None).await?;
    println!("File downloaded to {}", output_path);
    
    Ok(())
}
```

## Feature Examples

### Authentication

```rust
// Create a client with authentication
let config = ClientConfig {
    base_url: "http://localhost:8080".to_string(),
    credentials: Some(Credentials {
        username: "user".to_string(),
        password: "password".to_string(),
    }),
    timeout_seconds: 30,
};

let mut client = RfsClient::new(config);

// Authenticate with the server
client.authenticate().await?;
if client.is_authenticated() {
    println!("Authentication successful");
}
```

### File Management

```rust
// Upload a file with options
let upload_options = UploadOptions {
    chunk_size: Some(1024 * 1024), // 1MB chunks
    verify: true,
};

let file_hash = client.upload_file("/path/to/file.txt", Some(upload_options)).await?;

// Download the file
let download_options = DownloadOptions {
    verify: true,
};

client.download_file(&file_hash, "/path/to/output.txt", Some(download_options)).await?;
```

### FList Operations

```rust
// Create an FList from a Docker image
let options = FlistOptions {
    auth: None,
    username: None,
    password: None,
    email: None,
    server_address: Some("docker.io".to_string()),
    identity_token: None,
    registry_token: None,
};

let job_id = client.create_flist("alpine:latest", Some(options)).await?;

// Wait for FList creation with progress tracking
let wait_options = WaitOptions {
    timeout_seconds: 60,
    poll_interval_ms: 1000,
    progress_callback: Some(Box::new(|state| {
        println!("Progress: FList state is now {:?}", state);
    })),
};

let final_state = client.wait_for_flist_creation(&job_id, Some(wait_options)).await?;

// List available FLists
let flists = client.list_flists().await?;

// Preview an FList
let preview = client.preview_flist("flists/user/alpine-latest.fl").await?;

// Download an FList
client.download_flist("flists/user/alpine-latest.fl", "/tmp/downloaded_flist.fl").await?;
```

### Block Management

```rust
// List blocks
let blocks_list = client.list_blocks(None).await?;

// Check if a block exists
let exists = client.check_block("block_hash").await?;

// Get block content
let block_content = client.get_block("block_hash").await?;

// Upload a block
let block_hash = client.upload_block("file_hash", 0, data).await?;

// Verify blocks
let request = VerifyBlocksRequest { blocks: verify_blocks };
let verify_result = client.verify_blocks(request).await?;
```

## Complete Examples

For more detailed examples, check the `examples` directory:

- `authentication.rs`: Authentication and health check examples
- `file_management.rs`: File upload and download with verification
- `flist_operations.rs`: Complete FList creation, monitoring, listing, preview, and download
- `block_management.rs`: Block-level operations including listing, verification, and upload
- `wait_for_flist.rs`: Advanced FList creation with progress monitoring

Run an example with:

```bash
cargo run --example flist_operations
```

## Development

This library wraps the OpenAPI-generated client located in the `openapi` directory. The OpenAPI client was generated using the OpenAPI Generator CLI.

To build the library:

```bash
cargo build
```

To run tests:

```bash
cargo test
```

## License

MIT
