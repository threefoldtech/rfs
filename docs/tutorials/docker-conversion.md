# Converting Docker Images to Flists

This tutorial will guide you through the process of converting Docker images to flists using the RFS tool. This allows you to distribute and run containerized applications efficiently using the RFS system.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- Docker installed and running
- A basic understanding of Docker concepts

## Introduction to Docker Conversion

Converting a Docker image to an flist extracts the filesystem from the Docker image and creates an flist that represents it. This allows you to:

- Distribute Docker-based applications without requiring Docker
- Mount Docker image filesystems directly
- Leverage RFS's efficient storage and distribution mechanisms

## Basic Docker Conversion

### Converting a Public Docker Image

The simplest way to convert a Docker image to an flist is using the `docker` subcommand of the RFS tool:

```bash
rfs docker -i alpine:latest -s dir:///tmp/store
```

This command:
- Pulls the `alpine:latest` image from Docker Hub
- Extracts its filesystem
- Creates an flist named `alpine-latest.fl` in the current directory
- Stores the content in a directory store at `/tmp/store`

### Understanding the Output

After running the command, you should see output similar to:

```
Pulling image: alpine:latest
Extracting image layers
Processing filesystem
Found 236 files, 42 directories
Processed 236 files, 5.8 MB total
Created 124 unique blocks, 5.5 MB total
Flist created successfully: alpine-latest.fl
```

The resulting flist contains:
- The complete filesystem from the Docker image
- File metadata (permissions, ownership, timestamps)
- References to the content blocks in the store

## Advanced Docker Conversion

### Using Private Docker Images

To convert a private Docker image, you need to provide authentication information:

```bash
rfs docker -i private-registry.example.com/myapp:latest \
  -s dir:///tmp/store \
  --username myuser \
  --password mypassword
```

You can also use other authentication methods:

```bash
# Using an auth string
rfs docker -i private-registry.example.com/myapp:latest \
  -s dir:///tmp/store \
  --auth "base64-encoded-auth-string"

# Using a registry token
rfs docker -i private-registry.example.com/myapp:latest \
  -s dir:///tmp/store \
  --registry-token "registry-token"
```

### Using Multiple Storage Backends

As with regular flist creation, you can use multiple storage backends:

```bash
# Sharding across two stores
rfs docker -i nginx:latest \
  -s 00-80=dir:///tmp/store1 \
  -s 81-ff=dir:///tmp/store2

# Using a remote ZDB backend
rfs docker -i redis:latest \
  -s zdb://zdb.example.com:9900/namespace
```

## Practical Examples

### Converting a Web Server Image

```bash
# Convert the nginx image
rfs docker -i nginx:latest -s dir:///tmp/nginx-store
```

### Converting a Database Image

```bash
# Convert the PostgreSQL image
rfs docker -i postgres:13 -s dir:///tmp/postgres-store
```

### Converting a Custom Application Image

```bash
# Convert a custom application image
rfs docker -i myregistry.example.com/myapp:1.0 \
  -s dir:///tmp/myapp-store \
  --username myuser \
  --password mypassword
```

## Using Converted Flists

Once you've converted a Docker image to an flist, you can use it like any other flist:

### Mounting the Flist

```bash
# Create a mount point
mkdir -p ~/mount-point

# Mount the flist
sudo rfs mount -m alpine-latest.fl -c ~/rfs-cache ~/mount-point
```

### Exploring the Docker Image Filesystem

```bash
# List the contents of the mount point
ls -la ~/mount-point

# Explore the filesystem
find ~/mount-point -type f | sort
```

### Running Commands in the Docker Image Filesystem

You can use `chroot` to run commands in the Docker image filesystem:

```bash
# Run a command in the Docker image filesystem
sudo chroot ~/mount-point /bin/sh -c "ls -la"
```

## Advanced Topics

### Converting Multi-Architecture Images

Docker supports multi-architecture images, which contain variants for different CPU architectures. When converting such images, RFS will use the variant that matches your current architecture.

If you need to convert a specific architecture variant, you can use Docker's platform option:

```bash
# Pull the ARM64 variant
docker pull --platform linux/arm64 nginx:latest

# Convert the pulled image
rfs docker -i nginx:latest -s dir:///tmp/store
```

### Handling Docker Volumes

Docker volumes are not included in the image filesystem and are not converted to the flist. If your application relies on volumes, you'll need to handle them separately.

### Preserving Docker Metadata

The conversion process preserves some Docker metadata, such as environment variables and entry points, as tags in the flist:

```bash
# List the tags in the flist
rfs config -m alpine-latest.fl tag list
```

You might see tags like:
- `docker:env`: Environment variables
- `docker:cmd`: Default command
- `docker:entrypoint`: Entry point
- `docker:workdir`: Working directory

## Troubleshooting

### Image Pull Failures

If you encounter issues pulling the Docker image:

1. Check your internet connection
2. Verify that the image name is correct
3. Ensure you have the necessary permissions to pull the image
4. Check Docker's authentication configuration

### Conversion Failures

If the conversion process fails:

1. Check that Docker is running
2. Ensure you have sufficient disk space
3. Check for errors in the Docker daemon logs
4. Try pulling the image manually first

## Next Steps

Now that you know how to convert Docker images to flists, you might want to learn:

- [Mounting and Using Flists](./mounting-flists.md)
- [Setting Up the RFS Server](./server-setup.md)
- [Using the RFS Server to Convert Docker Images](../user-guides/fl-server.md)