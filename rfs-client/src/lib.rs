// RFS Client - A client library for the Remote File System server
// This library wraps the OpenAPI-generated client to provide a more user-friendly interface

pub mod client;
pub mod error;
pub mod types;

pub use client::RfsClient;
pub use error::RfsError;

// Re-export types from the OpenAPI client that are commonly used
pub use openapi::models;
