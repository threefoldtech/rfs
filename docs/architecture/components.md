# RFS Command Components

This document provides detailed information about the main components of the RFS command and how they work together.

## The RFS Command Structure

The `rfs` command is a comprehensive tool that integrates multiple functionalities into a single command-line interface. It's organized into several logical components, all accessible through subcommands:

```
rfs
├── Core Functionality
│   ├── pack
│   ├── mount
│   ├── unpack
│   ├── clone
│   └── config
├── Docker Functionality
│   └── docker
├── Server Functionality
│   └── server
└── Client Functionality
    ├── upload
    ├── upload-dir
    ├── download
    ├── download-dir
    ├── exists
    ├── flist create
    └── website-publish
```

## 1. Core Functionality

The core functionality of the `rfs` command provides the fundamental operations for working with flists.

### Key Features

- **Flist Creation** (`rfs pack`): Processes directories and creates compact metadata representations (flists)
- **Mounting** (`rfs mount`): Implements FUSE-based mounting of flists as read-only filesystems
- **Extraction** (`rfs unpack`): Allows downloading and extracting flist contents to a local directory
- **Store Management** (`rfs config`): Manages storage backends and flist metadata
- **Data Cloning** (`rfs clone`): Copies data between storage backends

### Core Modules

- **fungi**: Handles the flist metadata format (reading/writing)
- **store**: Manages different storage backends (dir, zdb, s3, http)
- **cache**: Implements caching of downloaded content
- **fs**: Provides filesystem operations and FUSE integration

## 2. Docker Functionality

The Docker functionality (`rfs docker`) allows converting Docker images into flists.

### Key Features

- **Docker Image Extraction**: Pulls and extracts Docker images
- **Layer Processing**: Handles Docker image layers and filesystem differences
- **Metadata Preservation**: Preserves Docker image metadata in the resulting flist
- **Multi-Store Support**: Supports uploading to multiple storage backends

### Implementation

The Docker functionality leverages Docker's API to extract image contents and then processes them using the core modules to create flists. This integration allows users to easily convert Docker images to flists using the same command-line tool they use for other flist operations.

## 3. Server Functionality

The server functionality (`rfs server`) provides a REST API for managing flists.

### Key Features

- **User Authentication**: Supports JWT-based authentication
- **Flist Management**: APIs for creating, listing, and downloading flists
- **Docker Integration**: Supports creating flists from Docker images
- **Multi-User Support**: Manages flists for multiple users

### Components

- **Authentication Module**: Handles user authentication and authorization
- **Block Handlers**: Manages content blocks (chunks of file data)
- **File Handlers**: Manages file operations
- **Website Handlers**: Serves the web interface
- **Database Integration**: Stores user and flist metadata

## 4. Client Functionality

The client functionality provides commands for interacting with the server API.

### Key Features

- **File Upload** (`rfs upload`, `rfs upload-dir`): Upload files and directories to the server
- **File Download** (`rfs download`, `rfs download-dir`): Download files and directories from the server
- **File Verification** (`rfs exists`): Check if files exist on the server
- **Flist Creation** (`rfs flist create`): Create flists on the server
- **Website Publishing** (`rfs website-publish`): Publish websites to the server

### Implementation

The client functionality communicates with the server API to perform operations. It provides a command-line interface for operations that would otherwise require direct API calls.

## 5. Web Interface

When the server functionality is running, it serves a web interface that provides a graphical user interface for interacting with the server API.

### Key Features

- **User Authentication**: Login interface for accessing the system
- **Flist Creation**: Interface for creating flists from Docker images
- **Flist Management**: Listing, previewing, and downloading flists
- **User-Friendly Design**: Intuitive interface for managing flists

## Integration Between Components

The components of the `rfs` command work together to provide a complete system:

1. **Core + Docker**: The Docker functionality uses the core modules to create flists from Docker images.

2. **Server + Core**: The server functionality uses the core modules to handle flist operations.

3. **Client + Server**: The client functionality communicates with the server API to perform operations.

4. **Web Interface + Server**: The web interface communicates with the server API through HTTP requests.

5. **All Components + Storage Backends**: All components interact with various storage backends to store and retrieve content.

## Deployment Scenarios

The `rfs` command can be used in various deployment scenarios:

1. **Local Usage**: Using the core and Docker functionality locally for creating and mounting flists.

2. **Server Deployment**: Running the server functionality to provide a central service for managing flists.

3. **Client-Server Usage**: Using the client functionality to interact with a remote server.

4. **Web Interface Usage**: Using the web interface to interact with the server through a browser.

## Next Steps

For more information about how data flows through these components, see the [Data Flow Documentation](./data-flow.md).