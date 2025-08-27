# RFS Documentation

Welcome to the comprehensive documentation for the RFS (Remote File System) command-line tool. This documentation covers all aspects of using the `rfs` command, including its various subcommands, concepts, tutorials, and user guides.

## Table of Contents

### Architecture

The architecture documentation provides a high-level overview of the RFS system and how it works.

- [Architecture Overview](./architecture/overview.md) - High-level architecture of the RFS system
- [Components](./architecture/components.md) - How the different components work together
- [Data Flow](./architecture/data-flow.md) - How data flows through the system
- [Storage Backends](./architecture/storage-backends.md) - Details about the supported storage backends

### Concepts

The concepts documentation explains the core concepts and principles behind RFS.

- [Understanding Flists](./concepts/flists.md) - What are flists and how they work
- [Understanding Storage Backends](./concepts/stores.md) - How storage backends work
- [Understanding Caching](./concepts/caching.md) - How caching works in RFS
- [Understanding Sharding and Replication](./concepts/sharding.md) - How sharding and replication work

### Tutorials

The tutorials provide step-by-step guides for common tasks using the `rfs` command.

### Core Functionality

- [Getting Started](./tutorials/getting-started.md) - Installation and basic usage
- [Creating Flists](./tutorials/creating-flists.md) - How to create flists from directories
- [Mounting Flists](./tutorials/mounting-flists.md) - How to mount and use flists
- [End-to-End Flist Workflow](./tutorials/end-to-end-flist-workflow.md) - Complete workflow for creating and using flists

### Docker Integration

- [Docker Conversion](./tutorials/docker-conversion.md) - How to convert Docker images to flists
- [End-to-End Docker Workflow](./tutorials/end-to-end-docker-workflow.md) - Complete workflow for Docker conversion

### Server and Distribution

- [Server Setup](./tutorials/server-setup.md) - How to set up the server functionality
- [Website Publishing](./tutorials/website-publishing.md) - How to publish websites using RFS
- [Syncing Files](./tutorials/syncing-files.md) - How to sync files between RFS servers

### User Guides

The user guides provide detailed information about using specific features of the `rfs` command.

- [RFS Command Reference](./user-guides/rfs-cli.md) - Comprehensive command reference
- [RFS Server Guide](./user-guides/fl-server.md) - Server configuration and management
- [Web Interface Guide](./user-guides/frontend.md) - Using the web interface
- [Performance Tuning](./user-guides/performance-tuning.md) - Optimizing performance
- [Troubleshooting](./user-guides/troubleshooting.md) - Common issues and solutions

## Command Overview

The `rfs` command provides all the functionality you need to work with flists:

```bash
# Core functionality
rfs pack        # Create flists from directories
rfs mount       # Mount flists as filesystems
rfs unpack      # Extract flist contents to a directory
rfs clone       # Copy data between stores
rfs config      # Manage flist metadata and stores

# Docker conversion
rfs docker      # Convert Docker images to flists

# Server functionality
rfs server      # Run the RFS server for web-based management

# Server API interaction
rfs upload      # Upload a file to a server
rfs upload-dir  # Upload a directory to a server
rfs download    # Download a file from a server
rfs download-dir # Download a directory from a server
rfs exists      # Check if a file exists on a server
rfs flist create # Create an flist on a server
rfs website-publish # Publish a website to a server
```

For detailed information about each command, use the `--help` flag:

```bash
rfs --help
rfs pack --help
rfs mount --help
```

## Getting Help

If you encounter issues or have questions that aren't addressed in the documentation:

1. Check the [Troubleshooting Guide](./user-guides/troubleshooting.md) for common issues and solutions.
2. Use the `--help` flag with any command for detailed usage information.
3. Search for similar issues on the [RFS GitHub repository](https://github.com/threefoldtech/rfs/issues).
4. Open a new issue if needed.

## Contributing to Documentation

We welcome contributions to improve this documentation. If you find errors, omissions, or areas that could be clarified:

1. Fork the repository
2. Make your changes
3. Submit a pull request

Please follow the existing style and structure when making changes.

## License

This documentation is licensed under the [Apache License 2.0](../LICENSE).
