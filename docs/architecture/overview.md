# RFS Architecture Overview

## Introduction

The Remote File System (RFS) is a distributed file system solution designed to efficiently store, distribute, and access file system data across various storage backends. It uses a compact metadata format called FungiList (FL) to represent file system structures while storing the actual data in configurable storage backends.

## The RFS Command

The `rfs` command is a comprehensive tool that provides all the functionality needed to work with flists. It integrates several capabilities:

1. **Core Functionality**: Creating, mounting, and extracting flists
2. **Docker Conversion**: Converting Docker images into flists
3. **Server Functionality**: Running a server that provides a REST API for managing flists
4. **Client Functionality**: Interacting with the server API

All these capabilities are accessible through various subcommands of the `rfs` command, providing a unified interface for all operations.

## High-Level Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   User      │     │  Frontend   │     │  RFS Server │
│  Interface  │────▶│  (Web UI)   │────▶│ Functionality│
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────────────────────────────────┐
│   Storage   │◀───▶│                RFS                      │
│  Backends   │     │ (Core, Docker, Server, Client Features) │
└─────────────┘     └─────────────────────────────────────────┘
```

## Command Structure

The `rfs` command is organized into several subcommands:

- **Core Commands**: `pack`, `mount`, `unpack`, `clone`, `config`
- **Docker Commands**: `docker`
- **Server Commands**: `server`
- **Client Commands**: `upload`, `upload-dir`, `download`, `download-dir`, `exists`, `flist create`, `website-publish`

Each subcommand provides specific functionality while sharing common libraries and approaches.

## Data Flow

1. **Creation Flow**:
   - User selects a directory or Docker image to convert to an flist using the appropriate `rfs` subcommand
   - RFS processes the files, creating metadata and storing content in the configured storage backends
   - The resulting flist (.fl file) contains metadata and references to the stored content

2. **Consumption Flow**:
   - User mounts an flist using the `rfs mount` command
   - RFS reads the metadata from the flist
   - When files are accessed, RFS retrieves the content from the storage backends on-demand
   - Retrieved content is cached locally for improved performance

3. **Server Flow**:
   - User starts the server using the `rfs server` command
   - The server provides a REST API for managing flists
   - Users can interact with the server through the web interface or directly through the API
   - The server uses the same core functionality as the command-line tool

## Design Principles

1. **Unified Interface**: All functionality is accessible through the `rfs` command, providing a consistent user experience.

2. **Separation of Metadata and Content**: By separating file metadata from content, RFS enables efficient distribution of file system structures while minimizing data transfer.

3. **Storage Backend Flexibility**: Support for multiple storage backends (directory, ZDB, S3, HTTP) allows for flexible deployment scenarios.

4. **On-Demand Access**: Files are only retrieved when accessed, reducing bandwidth and storage requirements.

5. **Caching**: Local caching of accessed files improves performance for frequently used data.

6. **Sharding and Replication**: Content can be sharded across multiple storage backends and replicated for redundancy.

## Next Steps

For more detailed information about specific aspects of the RFS architecture, refer to:

- [Components Documentation](./components.md)
- [Data Flow Documentation](./data-flow.md)
- [Storage Backends Documentation](./storage-backends.md)