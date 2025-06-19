# RFS Client

A Rust client library for interacting with the Remote File System (RFS) server.

## Overview

This client library provides a user-friendly wrapper around the OpenAPI-generated client code. It offers high-level abstractions for common operations such as:

- Authentication
- File uploads and downloads
- Block management
- FList creation and management

## Structure

The library is organized as follows:

- `client.rs`: Main client implementation with methods for interacting with the RFS server
- `error.rs`: Error types and handling
- `types.rs`: Type definitions and utilities

## Usage

```rust
use rfs_client::{RfsClient, ClientConfig, Credentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client with custom configuration
    let config = ClientConfig {
        base_url: "https://rfs-server.example.com".to_string(),
        credentials: Some(Credentials {
            username: "user".to_string(),
            password: "pass".to_string(),
        }),
        timeout_seconds: 60,
    };
    
    let mut client = RfsClient::new(config);
    
    // Authenticate
    client.authenticate().await?;
    
    // Get system info
    let system_info = client.get_system_info().await?;
    println!("System info: {}", system_info);
    
    // Create a new FList
    let flist_id = client.create_flist("my-flist", Some("My test FList")).await?;
    println!("Created FList with ID: {}", flist_id);
    
    Ok(())
}
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
