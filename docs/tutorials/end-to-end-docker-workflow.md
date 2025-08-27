# End-to-End Docker to Flist Workflow Tutorial

This tutorial will guide you through a complete workflow for converting Docker images to flists, exploring them, and using them with RFS. You'll learn how to convert Docker images to flists, inspect their contents, and mount them as filesystems.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- Docker installed and running
- Basic understanding of Docker and command-line operations

## Step 1: Choose a Docker Image

For this tutorial, we'll use the Alpine Linux image, which is small and widely used:

```bash
# Pull the Alpine image to ensure it's available locally
docker pull alpine:latest
```

## Step 2: Create a Store Directory

RFS needs a store to save the content blocks. Let's create a directory for this purpose:

```bash
mkdir -p ~/rfs-store
```

## Step 3: Convert the Docker Image to an Flist

Now, let's convert the Alpine Docker image to an flist:

```bash
rfs docker -i alpine:latest -s dir://~/rfs-store
```

You should see output similar to:

```
Pulling image: alpine:latest
Extracting image layers
Processing filesystem
Found 236 files, 42 directories
Processed 236 files, 5.8 MB total
Created 124 unique blocks, 5.5 MB total
Flist created successfully: alpine-latest.fl
```

The flist file `alpine-latest.fl` will be created in your current directory.

## Step 4: Explore the Flist

Let's explore the contents of the flist we just created:

```bash
# View the flist as a tree structure
rfs flist tree alpine-latest.fl
```

You should see output showing the directory structure of the Alpine Linux filesystem:

```
📁 bin
  📄 ash
  📄 busybox
  ...
📁 etc
  📄 passwd
  📄 group
  ...
📁 lib
  ...
```

Now, let's inspect the flist to see detailed information about all files and directories:

```bash
# Inspect the flist
rfs flist inspect alpine-latest.fl
```

This will show detailed metadata for each file and directory, followed by a summary:

```
Path: /bin/busybox
  Type: Regular File
  Inode: 12345
  Name: busybox
  Size: 848952 bytes
  UID: 0
  GID: 0
  Mode: 0100755
  Permissions: 0755
  Device: 0
  Created: 1609459200
  Modified: 1609459200
  ---
...

Flist Inspection: alpine-latest.fl
==================
Files: 236
Directories: 42
Symlinks: 15
Total size: 5834752 bytes
```

## Step 5: Check Docker Metadata

The Docker image metadata is preserved in the flist as tags. Let's check these tags:

```bash
# List tags
rfs config -m alpine-latest.fl tag list
```

You should see tags like:

```
docker:env=PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
docker:cmd=/bin/sh
docker:workdir=/
```

## Step 6: Mount the Flist

Now, let's mount the flist as a filesystem:

```bash
# Create a mount point
mkdir -p ~/alpine-mount

# Mount the flist
sudo rfs mount -m alpine-latest.fl -c ~/rfs-cache ~/alpine-mount
```

## Step 7: Use the Mounted Filesystem

Let's explore and use the mounted filesystem:

```bash
# List the contents of the mount point
ls -la ~/alpine-mount

# View the /etc/os-release file
cat ~/alpine-mount/etc/os-release

# Run a command in the Alpine environment using chroot
sudo chroot ~/alpine-mount /bin/sh -c "ls -la"
```

You can also run a shell in the Alpine environment:

```bash
sudo chroot ~/alpine-mount /bin/sh
```

This gives you a shell in the Alpine environment, where you can run commands as if you were in an Alpine container.

## Step 8: Unmount the Filesystem

When you're done, exit the chroot environment (if you entered it) and unmount the filesystem:

```bash
# Exit the chroot environment (if you entered it)
exit

# Unmount the filesystem
sudo umount ~/alpine-mount
```

## Step 9: Convert a More Complex Docker Image

Let's try converting a more complex Docker image, such as Nginx:

```bash
# Pull the Nginx image
docker pull nginx:latest

# Convert it to an flist
rfs docker -i nginx:latest -s dir://~/rfs-store
```

This will create `nginx-latest.fl` in your current directory.

## Step 10: Mount and Test the Nginx Flist

Let's mount the Nginx flist and verify it works:

```bash
# Create a mount point
mkdir -p ~/nginx-mount

# Mount the flist
sudo rfs mount -m nginx-latest.fl -c ~/rfs-cache ~/nginx-mount

# Check the Nginx binary
ls -la ~/nginx-mount/usr/sbin/nginx
```

## Step 11: Using Private Docker Images

If you need to convert a private Docker image, you can provide authentication information:

```bash
# Using username and password
rfs docker -i private-registry.example.com/myapp:latest \
  -s dir://~/rfs-store \
  --username myuser \
  --password mypassword
```

## Step 12: Clean Up

When you're done, you can clean up:

```bash
# Unmount the Nginx filesystem (if still mounted)
sudo umount ~/nginx-mount

# Remove the mount points
rmdir ~/alpine-mount ~/nginx-mount

# Remove the cache
rm -rf ~/rfs-cache

# Keep the flists and store if you want to use them later
```

## Advanced: Using Multiple Storage Backends

For production use, you might want to use multiple storage backends for redundancy or sharding:

```bash
# Sharding across two stores
mkdir -p ~/rfs-store1 ~/rfs-store2

rfs docker -i ubuntu:latest \
  -s 00-80=dir://~/rfs-store1 \
  -s 81-ff=dir://~/rfs-store2
```

## Next Steps

Now that you've completed this end-to-end workflow, you might want to:

- Learn about [advanced flist creation options](./creating-flists.md)
- Set up the [RFS Server](./server-setup.md) for web-based management
- Explore [storage backends](../concepts/stores.md) for distributed storage
- Learn about [sharding and replication](../concepts/sharding.md)

## Troubleshooting

### Docker Image Pull Fails

If pulling the Docker image fails:

1. Check your internet connection
2. Verify that the image name is correct
3. Ensure you have the necessary permissions to pull the image
4. Check Docker's authentication configuration

### Mount Fails with Permission Error

If mounting fails with a permission error, make sure you're using `sudo` for the mount command.

### Docker Conversion Fails

If the conversion process fails:

1. Check that Docker is running
2. Ensure you have sufficient disk space
3. Check for errors in the Docker daemon logs
4. Try pulling the image manually first