# RFS - Remote File System

[![Test](https://github.com/threefoldtech/rfs/actions/workflows/tests.yaml/badge.svg?branch=master)](https://github.com/threefoldtech/rfs/actions/workflows/tests.yaml)

## What is RFS?

RFS (Remote File System) is a powerful command-line tool that lets you efficiently store, distribute, and access filesystems across different storage backends. It solves common problems in file distribution by separating metadata from content, allowing for:

- **Efficient file distribution** - Share only the metadata, download content on-demand
- **Reduced bandwidth usage** - Only download the files you actually need
- **Flexible storage options** - Store content in local directories, ZDB, S3, or HTTP backends
- **Docker image conversion** - Convert Docker images to lightweight, mountable filesystems
- **Web-based management** - Manage your filesystems through a user-friendly web interface

## Key Features

- **Flists**: Compact metadata files that describe filesystems without containing the actual data
- **On-demand access**: Files are only downloaded when accessed, saving bandwidth and storage
- **Content deduplication**: Identical files are stored only once, even across different flists
- **Multiple storage backends**: Store content in directory, ZDB, S3, or HTTP backends
- **Docker conversion**: Convert Docker images to flists for efficient distribution
- **Server functionality**: Run a server for web-based flist management
- **Sharding and replication**: Distribute and replicate content across multiple storage backends

## Common Use Cases

### Efficient Docker Image Distribution

Convert Docker images to flists for more efficient distribution and usage:

```bash
# Convert a Docker image to an flist
rfs docker -i nginx:latest -s dir:///tmp/store

# Mount the resulting flist
sudo rfs mount -m nginx-latest.fl /mnt/nginx
```

### Distributing Large Filesystems

Package and distribute large filesystems efficiently:

```bash
# Create an flist from a directory
rfs pack -m myapp.fl -s dir:///tmp/store /path/to/myapp

# Share the flist (typically <1MB) instead of the entire content
# Recipients can mount it and access files on-demand
sudo rfs mount -m myapp.fl /mnt/myapp
```

### Web Content Distribution

Publish and distribute web content efficiently:

```bash
# Create an flist from a website directory
rfs pack -m website.fl -s s3://user:pass@s3.example.com:9000/bucket /path/to/website

# Mount the website on a server
sudo rfs mount -m website.fl /var/www/html
```

### Application Deployment

Deploy applications with all their dependencies:

```bash
# Convert an application Docker image to an flist
rfs docker -i myapp:latest -s zdb://zdb.example.com:9900/namespace

# Deploy the application on multiple servers by mounting the flist
sudo rfs mount -m myapp-latest.fl /opt/myapp
```

## Quick Start

### Installation

```bash
# Install dependencies
sudo apt-get install -y build-essential fuse libfuse-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone the repository
git clone https://github.com/threefoldtech/rfs.git
cd rfs

# Build RFS
rustup target add x86_64-unknown-linux-musl
cargo build --features build-binary --release --target=x86_64-unknown-linux-musl

# Install the binary
sudo cp ./target/x86_64-unknown-linux-musl/release/rfs /usr/local/bin/
```

### Creating Your First Flist

```bash
# Create a directory to use as a store
mkdir -p ~/rfs-store

# Create an flist from a directory
rfs pack -m ~/test.fl -s dir://~/rfs-store ~/my-directory

# Mount the flist
mkdir -p ~/mount-point
sudo rfs mount -m ~/test.fl ~/mount-point

# Access the files
ls -la ~/mount-point

# Unmount when done
sudo umount ~/mount-point
```

### Converting a Docker Image

```bash
# Create a directory to use as a store
mkdir -p ~/rfs-store

# Convert a Docker image to an flist
rfs docker -i alpine:latest -s dir://~/rfs-store

# Mount the resulting flist
mkdir -p ~/mount-point
sudo rfs mount -m alpine-latest.fl ~/mount-point

# Run commands in the mounted filesystem
sudo chroot ~/mount-point /bin/sh -c "ls -la"

# Unmount when done
sudo umount ~/mount-point
```

### Running the Server

```bash
# Create a configuration file
cat > config.toml << EOF
host = "localhost"
port = 3000
store_url = ["dir:///tmp/store"]
flist_dir = "flists"

jwt_secret = "your-secret-key"
jwt_expire_hours = 5

[[users]]
username = "admin"
password = "admin-password"
EOF

# Create the flists directory
mkdir -p flists/admin

# Run the server
rfs server --config-path config.toml
```

## Command Reference

The `rfs` command provides all the functionality you need to work with flists:

- `rfs pack` - Create flists from directories
- `rfs mount` - Mount flists as filesystems
- `rfs unpack` - Extract flist contents to a directory
- `rfs docker` - Convert Docker images to flists
- `rfs server` - Run the RFS server for web-based management
- `rfs config` - Manage flist metadata and stores
- `rfs clone` - Copy data between stores
- `rfs flist tree` - Display flist contents as a tree structure
- `rfs flist inspect` - Inspect file details within an flist
- And more...

For detailed information about each command, use the `--help` flag:

```bash
rfs --help
rfs pack --help
rfs mount --help
```

## Documentation

Comprehensive documentation is available in the [docs](./docs) directory:

### Getting Started

- [Installation and Basic Usage](./docs/tutorials/getting-started.md)
### Core Functionality

- [Getting Started](./docs/tutorials/getting-started.md) - Installation and basic usage
- [Creating Flists](./docs/tutorials/creating-flists.md) - How to create flists from directories
- [Mounting Flists](./docs/tutorials/mounting-flists.md) - How to mount and use flists
- [End-to-End Flist Workflow](./docs/tutorials/end-to-end-flist-workflow.md) - Complete workflow for creating and using flists

### Docker Integration

- [Converting Docker Images](./docs/tutorials/docker-conversion.md) - How to convert Docker images to flists
- [End-to-End Docker Workflow](./docs/tutorials/end-to-end-docker-workflow.md) - Complete workflow for Docker conversion

### Server and Distribution

- [Server Setup](./docs/tutorials/server-setup.md) - How to set up the RFS server
- [Website Publishing](./docs/tutorials/website-publishing.md) - How to publish websites using RFS
- [Syncing Files](./docs/tutorials/syncing-files.md) - How to sync files between RFS servers

### User Guides

- [RFS Command Reference](./docs/user-guides/rfs-cli.md)
- [RFS Server Setup and Usage](./docs/user-guides/fl-server.md)
- [Web Interface Guide](./docs/user-guides/frontend.md)
- [Performance Tuning](./docs/user-guides/performance-tuning.md)
- [Troubleshooting](./docs/user-guides/troubleshooting.md)

### Concepts and Architecture

- [Understanding Flists](./docs/concepts/flists.md)
- [Storage Backends](./docs/concepts/stores.md)
- [Caching](./docs/concepts/caching.md)
- [Sharding and Replication](./docs/concepts/sharding.md)
- [System Architecture](./docs/architecture/overview.md)

## Community and Support

- **GitHub Issues**: Report bugs or request features on our [GitHub repository](https://github.com/threefoldtech/rfs/issues)
- **Documentation**: Comprehensive documentation is available in the [docs](./docs) directory

## License

This project is licensed under the [Apache License 2.0](./LICENSE).
