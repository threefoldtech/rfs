# RFS Command Data Flow

This document describes how data flows through the RFS command during various operations.

## 1. Creating an Flist (`rfs pack`)

The process of creating an flist from a directory using the `rfs pack` command involves several steps:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Source     │     │  Metadata   │     │  Content    │     │  Storage    │
│  Directory  │────▶│  Extraction │────▶│  Chunking   │────▶│  Backend    │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                          │                                       ▲
                          │                                       │
                          ▼                                       │
                   ┌─────────────┐                        ┌─────────────┐
                   │   Flist     │                        │  Content    │
                   │  Creation   │───────────────────────▶│  Upload     │
                   └─────────────┘                        └─────────────┘
```

### Step-by-Step Process

1. **Directory Scanning**:
   - The `rfs pack` command recursively scans the source directory
   - Collects metadata for each file and directory (permissions, ownership, timestamps, etc.)

2. **Content Processing**:
   - For each file, RFS reads the content
   - Splits large files into chunks (default chunk size is configurable)
   - Calculates a hash for each chunk

3. **Deduplication**:
   - Identifies duplicate chunks based on their hashes
   - Only unique chunks are stored, reducing storage requirements

4. **Storage Upload**:
   - Uploads unique chunks to the configured storage backend(s)
   - If multiple storage backends are configured, chunks may be sharded or replicated

5. **Flist Creation**:
   - Creates a metadata file (.fl) containing:
     - Directory structure
     - File metadata
     - References to content chunks in storage
     - Storage backend information

## 2. Mounting an Flist (`rfs mount`)

When mounting an flist using the `rfs mount` command, RFS creates a virtual filesystem that fetches content on-demand:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Flist     │     │  Metadata   │     │   FUSE      │
│    File     │────▶│   Loading   │────▶│  Mounting   │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Storage   │     │   Content   │     │ File Access │
│   Backend   │◀───▶│  Retrieval  │◀───▶│  Request    │
└─────────────┘     └─────────────┘     └─────────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │    Local    │
                   │    Cache    │
                   └─────────────┘
```

### Step-by-Step Process

1. **Flist Loading**:
   - The `rfs mount` command reads the flist file
   - Parses the metadata and directory structure
   - Identifies the storage backends

2. **FUSE Mount Setup**:
   - Creates a FUSE mount point
   - Sets up filesystem operations (read, readdir, getattr, etc.)
   - Presents the directory structure to the operating system

3. **On-Demand Content Retrieval**:
   - When a file is accessed, RFS:
     - Identifies the chunks needed for the requested file portion
     - Retrieves the chunks from the storage backend
     - Caches the chunks locally
     - Assembles the file content and returns it to the application

4. **Caching**:
   - Retrieved chunks are stored in a local cache
   - Subsequent access to the same file uses the cached content
   - Cache size and expiration are configurable

## 3. Converting Docker Images (`rfs docker`)

Converting a Docker image to an flist using the `rfs docker` command involves additional steps:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Docker    │     │   Image     │     │   Layer     │
│   Registry  │────▶│   Pull      │────▶│  Extraction │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Flist     │     │  Metadata   │     │  Filesystem │
│  Creation   │◀───▶│  Extraction │◀───▶│  Assembly   │
└─────────────┘     └─────────────┘     └─────────────┘
       │
       ▼
┌─────────────┐
│  Standard   │
│ Pack Process│
└─────────────┘
```

### Step-by-Step Process

1. **Docker Image Pulling**:
   - The `rfs docker` command pulls the specified Docker image from a registry
   - Authenticates if credentials are provided

2. **Layer Extraction**:
   - Extracts each layer of the Docker image
   - Applies layer diffs to create a complete filesystem

3. **Filesystem Assembly**:
   - Combines the layers according to Docker's overlay filesystem rules
   - Resolves file conflicts between layers

4. **Metadata Extraction**:
   - Extracts Docker-specific metadata (environment variables, entry points, etc.)
   - Preserves this metadata in the flist

5. **Flist Creation**:
   - Proceeds with the standard pack process as described earlier
   - Creates an flist representing the Docker image's filesystem

## 4. Running the Server (`rfs server`)

When running the server using the `rfs server` command, RFS provides a REST API for managing flists:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │     │  REST API   │     │ Auth/User   │
│   Request   │────▶│  Endpoint   │────▶│  Validation │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Response  │     │  Operation  │     │ RFS Core    │
│    to User  │◀───▶│   Result    │◀───▶│ Functionality│
└─────────────┘     └─────────────┘     └─────────────┘
```

### Common Operations

1. **User Authentication**:
   - Validates user credentials
   - Issues JWT tokens for authenticated sessions

2. **Flist Creation**:
   - Accepts Docker image information or uploaded files
   - Uses the same core functionality as the `rfs pack` and `rfs docker` commands
   - Stores the resulting flist in the user's directory

3. **Flist Listing and Retrieval**:
   - Lists flists available to the user
   - Provides download links for flists

4. **Block Management**:
   - Handles upload and download of content blocks
   - Manages block storage and retrieval

## 5. Client-Server Interaction

The client subcommands (`rfs upload`, `rfs download`, etc.) communicate with a running server:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ RFS Client  │     │  HTTP/REST  │     │ RFS Server  │
│  Command    │────▶│   Request   │────▶│  Endpoint   │
└─────────────┘     └─────────────┘     └─────────────┘
       ▲                                       │
       │                                       │
       │                                       ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Command   │     │   Response  │     │  Server     │
│   Output    │◀───▶│  Processing │◀───▶│  Processing │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Common Client Commands

1. **File Upload** (`rfs upload`, `rfs upload-dir`):
   - Reads file(s) from the local filesystem
   - Splits into chunks if necessary
   - Sends chunks to the server
   - Receives confirmation from the server

2. **File Download** (`rfs download`, `rfs download-dir`):
   - Sends request for file(s) to the server
   - Receives chunks from the server
   - Assembles chunks into complete file(s)
   - Writes to the local filesystem

3. **File Verification** (`rfs exists`):
   - Sends hash or file information to the server
   - Receives existence confirmation from the server

4. **Flist Creation** (`rfs flist create`):
   - Uploads directory content to the server
   - Requests flist creation from the server
   - Receives the resulting flist

## 6. Web Interface Interaction

When the server is running, it also serves a web interface that communicates with the server API:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Browser   │     │  Frontend   │     │  REST API   │
│   User      │────▶│   Action    │────▶│   Call      │
└─────────────┘     └─────────────┘     └─────────────┘
       ▲                                       │
       │                                       │
       │                                       ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    UI       │     │   Response  │     │  Server     │
│   Update    │◀───▶│  Processing │◀───▶│  Processing │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Common Interactions

1. **User Login**:
   - User enters credentials in the web interface
   - Frontend sends authentication request to the server
   - Stores JWT token for subsequent requests

2. **Flist Creation**:
   - User specifies Docker image or uploads files through the web interface
   - Frontend sends creation request to the server
   - Displays progress and result to the user

3. **Flist Management**:
   - Lists user's flists
   - Provides preview functionality
   - Enables download of flists

## Data Sharing Between Commands

All RFS commands share common code and data structures, ensuring consistent behavior:

1. **Storage Backend Handling**: All commands that interact with storage backends use the same code for accessing and manipulating content.

2. **Flist Format**: All commands that read or write flists use the same code for parsing and generating the flist format.

3. **Caching**: Commands that read content from storage backends share the same caching mechanism.

4. **Configuration**: Commands share common configuration handling for settings like parallel downloads, cache location, etc.

## Next Steps

For more information about the storage backends and how they interact with the system, see the [Storage Backends Documentation](./storage-backends.md).