# RFS CLI User Guide

This comprehensive guide covers all the commands and options available in the RFS command-line interface (CLI).

## Overview

The RFS CLI provides a set of commands for working with flists, including creating, mounting, and extracting them. It also includes commands for managing storage backends, converting Docker images, and running the FL server.

## Global Options

These options apply to all RFS commands:

- `--help`, `-h`: Display help information for a command
- `--version`, `-V`: Display the version of RFS
## Core Commands

### pack

Creates an FL (flist) and uploads blocks to the provided storage.

```bash
rfs pack [OPTIONS] --meta <META> --store <STORE>... <TARGET>
```

**Options:**

- `--meta`, `-m`: Path to the metadata file (flist)
- `--store`, `-s`: Store URL(s) in the format `[xx-xx=]<url>`. Multiple stores can be specified.
- `--no-strip-password`: Disable automatic password stripping from the store URL.
- `<TARGET>`: Directory to upload.

**Example:**
```bash
rfs pack -m output.fl -s dir:///tmp/store ~/Documents
```

### mount

Mount an FL to a target directory.

```bash
rfs mount [OPTIONS] --meta <META> <TARGET>
```

**Options:**

- `--meta`, `-m`: Path to the metadata file (flist)
- `--cache`, `-c`: Directory used as cache for downloaded file chunks (default: `/tmp/cache`)
- `--daemon`, `-d`: Run in the background
- `--log`, `-l`: Log file (only used with daemon mode)
- `<TARGET>`: Target mount point

**Example:**
```bash
sudo rfs mount -m output.fl -c ~/cache ~/mount-point
```

### unpack

Unpack (download) content of an FL to the provided location.

```bash
rfs unpack [OPTIONS] --meta <META> <TARGET>
```

**Options:**

- `--meta`, `-m`: Path to the metadata file (flist)
- `--cache`, `-c`: Directory used as cache for downloaded file chunks (default: `/tmp/cache`)
- `--preserve-ownership`, `-p`: Preserve files ownership from the FL (requires sudo)
- `<TARGET>`: Target directory to unpack to

**Example:**
```bash
rfs unpack -m output.fl -c ~/cache ~/extracted
```

### clone

Copy data from the stores of an FL to another store.

```bash
rfs clone [OPTIONS] --meta <META> --store <STORE>...
```

**Options:**

- `--meta`, `-m`: Path to the metadata file (flist)
- `--store`, `-s`: Store URL(s) in the format `[xx-xx=]<url>`. Multiple stores can be specified.
- `--cache`, `-c`: Directory used as cache for downloaded file chunks (default: `/tmp/cache`)

**Example:**
```bash
rfs clone -m output.fl -s dir:///tmp/new-store -c ~/cache
```

### config

List or modify FL metadata and stores.

```bash
rfs config --meta <META> <SUBCOMMAND>
```

**Subcommands:**

- `tag list`: List all tags
- `tag add --tag <KEY=VALUE>`: Add a tag
- `tag delete --key <KEY>`: Delete a tag
- `store list`: List all stores
- `store add --store <STORE>`: Add a store
- `store delete --store <STORE>`: Delete a store

**Example:**
```bash
rfs config -m output.fl tag list
```

### docker

Convert a Docker image to an FL.

```bash
rfs docker [OPTIONS] --image-name <IMAGE_NAME> --store <STORE>...
```

**Options:**

- `--image-name`, `-i`: Name of the Docker image to convert
- `--store`, `-s`: Store URL(s) in the format `[xx-xx=]<url>`. Multiple stores can be specified.
- `--username`: Username for Docker registry authentication
- `--password`: Password for Docker registry authentication
- `--auth`: Authentication string for Docker registry
- `--email`: Email for Docker registry authentication
- `--server-address`: Address of the Docker registry
- `--identity-token`: Identity token for Docker registry authentication
- `--registry-token`: Registry token for Docker registry authentication

**Example:**
```bash
rfs docker -i nginx:latest -s dir:///tmp/store
```

### server

Run the FL server.

```bash
rfs server [OPTIONS] --config-path <CONFIG_PATH>
```

**Options:**

- `--config-path`, `-c`: Path to the server configuration file
- `--debug`, `-d`: Enable debugging logs (can be specified multiple times for more verbose logging)

**Example:**
```bash
rfs server --config-path config.toml --debug
```

### upload

Upload a file to a server.

```bash
rfs upload [OPTIONS] <FILE_PATH> --server <SERVER>
```

**Options:**

- `<FILE_PATH>`: Path to the file to upload
- `--server`: Server URL (e.g., `http://localhost:8080`)
- `--block-size`: Block size for splitting the file (default: 1MB)

**Example:**
```bash
rfs upload large-file.bin --server http://localhost:8080 --block-size 2097152
```

### upload-dir

Upload a directory to a server.

```bash
rfs upload-dir [OPTIONS] <DIRECTORY_PATH> --server <SERVER>
```

**Options:**

- `<DIRECTORY_PATH>`: Path to the directory to upload
- `--server`: Server URL (e.g., `http://localhost:8080`)
- `--block-size`: Block size for splitting the files (default: 1MB)
- `--create-flist`: Create and upload an FL file
- `--flist-output`: Path to output the FL file

**Example:**
```bash
rfs upload-dir ./website --server http://localhost:8080 --create-flist
```

### download

Download a file from a server using its hash.

```bash
rfs download [OPTIONS] <FILE_HASH> --output <OUTPUT_FILE> --server <SERVER>
```

**Options:**

- `<FILE_HASH>`: Hash of the file to download
- `--output`: Name to save the downloaded file as
- `--server`: Server URL (e.g., `http://localhost:8080`)

**Example:**
```bash
rfs download abc123 --output downloaded-file.bin --server http://localhost:8080
```

### download-dir

Download a directory from a server using its FL hash.

```bash
rfs download-dir [OPTIONS] <FLIST_HASH> --output <OUTPUT_DIRECTORY> --server <SERVER>
```

**Options:**

- `<FLIST_HASH>`: Hash of the FL to download
- `--output`: Directory to save the downloaded files to
- `--server`: Server URL (e.g., `http://localhost:8080`)

**Example:**
```bash
rfs download-dir def456 --output ./downloaded-dir --server http://localhost:8080
```

### exists

Check if a file or hash exists on the server.

```bash
rfs exists [OPTIONS] <FILE_OR_HASH> --server <SERVER>
```

**Options:**

- `<FILE_OR_HASH>`: Path to the file or hash to check
- `--server`: Server URL (e.g., `http://localhost:8080`)
- `--block-size`: Block size for splitting the file (default: 1MB)

**Example:**
```bash
rfs exists myfilehash --server http://localhost:8080
```

### flist create

Creates an flist from a directory.

```bash
rfs flist create [OPTIONS] <DIRECTORY> --output <OUTPUT> --server <SERVER>
```

**Options:**

- `<DIRECTORY>`: Path to the directory to create the flist from
- `--output`: Path to save the generated flist file
- `--server`: Server URL (e.g., `http://localhost:8080`)
- `--block-size`: Block size for splitting the files (default: 1MB)

**Example:**
```bash
rfs flist create ./mydir --output ./mydir.flist --server http://localhost:8080
```

### flist tree

Displays the contents of an flist as a tree structure.

```bash
rfs flist tree <TARGET> [OPTIONS]
```

**Options:**

- `<TARGET>`: Path to the flist file or hash of a flist on a server
- `--server-url`: Server URL for hash-based operations (only needed when using a hash)

**Example:**
```bash
# Display contents of a local flist
rfs flist tree myapp.fl

# Display contents of a flist from a server using its hash
rfs flist tree abc123... --server-url http://localhost:8080
```

### flist inspect

Inspects the details of an flist, showing metadata for all files and directories.

```bash
rfs flist inspect <TARGET> [OPTIONS]
```

**Options:**

- `<TARGET>`: Path to the flist file or hash of a flist on a server
- `--server-url`: Server URL for hash-based operations (only needed when using a hash)

**Example:**
```bash
# Inspect a local flist
rfs flist inspect myapp.fl

# Inspect a flist from a server using its hash
rfs flist inspect abc123... --server-url http://localhost:8080
```

The output includes detailed information for each file and directory in the flist, such as:
- File type (regular file, directory, symlink, etc.)
- Size
- Permissions
- Ownership
- Timestamps
- Link targets (for symlinks)

At the end, a summary is displayed showing:
- Total number of files
- Total number of directories
- Total number of symlinks
- Total size of all files

### website-publish

Publish a website directory to the server.

```bash
rfs website-publish [OPTIONS] <DIRECTORY_PATH> --server <SERVER>
```

**Options:**

- `<DIRECTORY_PATH>`: Path to the website directory to publish
- `--server`: Server URL (e.g., `http://localhost:8080`)
- `--block-size`: Block size for splitting the files (default: 1MB)

**Example:**
```bash
rfs website-publish ./website --server http://localhost:8080
```

## Environment Variables

RFS behavior can be influenced by the following environment variables:

- `RFS_PARALLEL_DOWNLOAD`: Number of parallel downloads (default: determined automatically)
- `RFS_CACHE_DIR`: Default cache directory (overridden by `--cache` option)
- `RFS_DEBUG`: Enable debug logging when set to any value

## Exit Codes

RFS returns the following exit codes:

- `0`: Success
- `1`: General error
- `2`: Command-line argument error
- `3`: File or directory not found
- `4`: Permission denied
- `5`: Network error
- `6`: Storage backend error

## See Also

- [RFS Architecture](../architecture/overview.md)
- [Tutorials](../tutorials/)
- [Concepts](../concepts/)