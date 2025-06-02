# End-to-End Flist Workflow Tutorial

This tutorial will guide you through a complete workflow for creating, exploring, and using flists with RFS. You'll learn how to create flists from directories, inspect their contents, and mount them as filesystems.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- Basic understanding of file systems and command-line operations
- A directory with some files to convert to an flist

## Step 1: Prepare a Directory

First, let's create a sample directory with some files to convert to an flist:

```bash
# Create a sample directory structure
mkdir -p ~/sample-app/{bin,etc,var/log}

# Create some sample files
echo '#!/bin/bash\necho "Hello, World!"' > ~/sample-app/bin/hello
chmod +x ~/sample-app/bin/hello

echo 'name=sample-app' > ~/sample-app/etc/config
echo 'version=1.0.0' >> ~/sample-app/etc/config

# Create a sample log file
echo 'Initialization complete' > ~/sample-app/var/log/app.log
```

## Step 2: Create a Store Directory

RFS needs a store to save the content blocks. Let's create a directory for this purpose:

```bash
mkdir -p ~/rfs-store
```

## Step 3: Create an Flist

Now, let's create an flist from our sample directory:

```bash
rfs pack -m ~/sample-app.fl -s dir://~/rfs-store ~/sample-app
```

You should see output similar to:

```
Processing directory: /home/user/sample-app
Found 3 files, 4 directories
Processed 3 files, 1.2 KB total
Created 3 unique blocks, 1.2 KB total
Flist created successfully: /home/user/sample-app.fl
```

## Step 4: Explore the Flist

Let's explore the contents of the flist we just created:

```bash
# View the flist as a tree structure
rfs flist tree ~/sample-app.fl
```

You should see output similar to:

```
📁 bin
  📄 hello
📁 etc
  📄 config
📁 var
  📁 log
    📄 app.log
```

Now, let's inspect the flist to see detailed information about all files and directories:

```bash
# Inspect the flist
rfs flist inspect ~/sample-app.fl
```

This will show detailed metadata for each file and directory, followed by a summary:

```
Path: /bin/hello
  Type: Regular File
  Inode: 12345
  Name: hello
  Size: 31 bytes
  UID: 1000
  GID: 1000
  Mode: 0100755
  Permissions: 0755
  Device: 0
  Created: 1609459200
  Modified: 1609459200
  ---
...

Flist Inspection: /home/user/sample-app.fl
==================
Files: 3
Directories: 4
Symlinks: 0
Total size: 1200 bytes
```

## Step 5: Add Metadata to the Flist

Let's add some metadata tags to our flist:

```bash
# Add metadata tags
rfs config -m ~/sample-app.fl tag add -t app=sample-app
rfs config -m ~/sample-app.fl tag add -t version=1.0.0
rfs config -m ~/sample-app.fl tag add -t created=$(date +%Y-%m-%d)
```

Now, let's list the tags to verify they were added:

```bash
# List tags
rfs config -m ~/sample-app.fl tag list
```

You should see:

```
app=sample-app
version=1.0.0
created=2025-06-02
```

## Step 6: Mount the Flist

Now, let's mount the flist as a filesystem:

```bash
# Create a mount point
mkdir -p ~/mount-point

# Mount the flist
sudo rfs mount -m ~/sample-app.fl -c ~/rfs-cache ~/mount-point
```

## Step 7: Use the Mounted Filesystem

Let's explore and use the mounted filesystem:

```bash
# List the contents of the mount point
ls -la ~/mount-point

# View the config file
cat ~/mount-point/etc/config

# Run the hello script
~/mount-point/bin/hello
```

You should see the "Hello, World!" message when running the script.

## Step 8: Unmount the Filesystem

When you're done, unmount the filesystem:

```bash
sudo umount ~/mount-point
```

## Step 9: Extract the Flist to a Directory

If you want to extract the contents of the flist to a directory:

```bash
# Create a directory for extraction
mkdir -p ~/extracted-app

# Extract the flist
rfs unpack -m ~/sample-app.fl -c ~/rfs-cache ~/extracted-app
```

Now you can verify that the extracted files match the original:

```bash
# Compare the original and extracted directories
diff -r ~/sample-app ~/extracted-app
```

## Step 10: Clean Up

When you're done, you can clean up:

```bash
# Remove the mount point
rmdir ~/mount-point

# Remove the extracted directory
rm -rf ~/extracted-app

# Remove the cache
rm -rf ~/rfs-cache

# Keep the flist and store if you want to use them later
```

## Next Steps

Now that you've completed this end-to-end workflow, you might want to:

- Learn how to [convert Docker images to flists](./docker-conversion.md)
- Set up the [RFS Server](./server-setup.md) for web-based management
- Explore [advanced flist creation options](./creating-flists.md)
- Learn about [storage backends](../concepts/stores.md) for distributed storage

## Troubleshooting

### Mount Fails with Permission Error

If mounting fails with a permission error, make sure you're using `sudo` for the mount command.

### Files Not Accessible in Mounted Filesystem

If files in the mounted filesystem are not accessible, check:
- The flist was created correctly
- The store directory is accessible
- The cache directory is writable

### Flist Creation Fails

If flist creation fails, check:
- The source directory exists and is readable
- The store directory is writable
- You have sufficient disk space