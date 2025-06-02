# Creating Flists with RFS

This tutorial will guide you through the process of creating flists using the RFS tool. We'll cover creating flists from directories, configuring storage backends, and managing flist metadata.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- A basic understanding of RFS concepts (see the [Concepts](../concepts/flists.md) documentation)

## Basic Flist Creation

### Creating an Flist from a Directory

The most basic way to create an flist is from a directory:

```bash
rfs pack -m output.fl -s dir:///tmp/store /path/to/directory
```

This command:
- Creates an flist named `output.fl` in the current directory
- Uses a directory store at `/tmp/store` to store the file content
- Processes all files in `/path/to/directory` recursively

### Understanding the Output

After running the command, you should see output similar to:

```
Processing directory: /path/to/directory
Found 42 files, 5 directories
Processed 42 files, 10.5 MB total
Created 37 unique blocks, 10.2 MB total
Flist created successfully: output.fl
```

The flist file (`output.fl`) contains:
- The directory structure
- File metadata (permissions, ownership, timestamps)
- References to the content blocks in the store

## Advanced Flist Creation

### Using Multiple Storage Backends

You can use multiple storage backends to shard or replicate content:

```bash
# Sharding across two stores
rfs pack -m output.fl -s 00-80=dir:///tmp/store1 -s 81-ff=dir:///tmp/store2 /path/to/directory

# Replicating to two stores
rfs pack -m output.fl -s dir:///tmp/store1 -s dir:///tmp/store2 /path/to/directory
```

### Using Remote Storage Backends

For production use, you'll typically want to use a remote storage backend:

```bash
# Using a ZDB backend
rfs pack -m output.fl -s zdb://zdb.example.com:9900/namespace /path/to/directory

# Using an S3 backend
rfs pack -m output.fl -s s3://username:password@s3.example.com:9000/bucket /path/to/directory
```

### Password Handling

By default, RFS strips passwords from store URLs when creating flists. This is a security feature to prevent unauthorized write access. If you need to preserve the password, use the `--no-strip-password` flag:

```bash
rfs pack -m output.fl -s s3://username:password@s3.example.com:9000/bucket --no-strip-password /path/to/directory
```

**Warning**: Using `--no-strip-password` means that anyone with access to the flist can extract the password and potentially gain write access to your store.

## Managing Flist Metadata

### Adding Tags

You can add custom tags to an flist:

```bash
rfs config -m output.fl tag add -t key=value
```

Tags can be used to store arbitrary metadata about the flist, such as:
- Version information
- Creation date
- Author information
- Description

### Listing Tags

To list the tags in an flist:

```bash
rfs config -m output.fl tag list
```

### Deleting Tags

To delete a tag from an flist:

```bash
rfs config -m output.fl tag delete -k key
```

### Managing Stores

You can also manage the stores associated with an flist:

```bash
# List stores
rfs config -m output.fl store list

# Add a store
rfs config -m output.fl store add -s dir:///tmp/store3

# Delete a store
rfs config -m output.fl store delete -s dir:///tmp/store1
```

## Best Practices

### Organizing Content

When creating flists, consider how to organize your content:

- **Group related files**: Create flists for logically related sets of files
- **Consider size**: Very large flists may be unwieldy; consider splitting them
- **Consider usage patterns**: Group files that are likely to be accessed together

### Choosing Storage Backends

Choose storage backends based on your requirements:

- **Local development**: Use directory stores for simplicity
- **Production**: Use ZDB or S3 for reliability and accessibility
- **Distribution**: Ensure your storage backend is accessible to users of the flist

### Metadata and Documentation

Use tags to document your flists:

```bash
rfs config -m output.fl tag add -t version=1.0.0
rfs config -m output.fl tag add -t created=$(date +%Y-%m-%d)
rfs config -m output.fl tag add -t author="Your Name"
rfs config -m output.fl tag add -t description="Description of the flist"
```

## Examples

### Creating an Flist for a Web Application

```bash
# Create a directory store
mkdir -p /tmp/webapp-store

# Create an flist from the web application directory
rfs pack -m webapp.fl -s dir:///tmp/webapp-store /path/to/webapp

# Add metadata
rfs config -m webapp.fl tag add -t app=webapp
rfs config -m webapp.fl tag add -t version=1.0.0
rfs config -m webapp.fl tag add -t description="Web application flist"
```

### Creating an Flist with Multiple Stores

```bash
# Create an flist with content sharded across two ZDB instances
rfs pack -m distributed.fl \
  -s 00-7f=zdb://zdb1.example.com:9900/namespace \
  -s 80-ff=zdb://zdb2.example.com:9900/namespace \
  /path/to/directory
```

### Creating an Flist for Distribution

```bash
# Create an flist using an S3 backend with public read access
rfs pack -m public.fl -s s3://username:password@s3.example.com:9000/public-bucket /path/to/directory

# Add a public HTTP store for read access
rfs config -m public.fl store add -s http://s3.example.com:9000/public-bucket
```

## Exploring Flists

After creating an flist, you can explore its contents and inspect file details without mounting it.

### Viewing Flist Contents as a Tree

You can view the contents of an flist as a tree structure:

```bash
# Display contents of a local flist
rfs flist tree output.fl

# Display contents of a flist from a server using its hash
rfs flist tree abc123... --server-url http://localhost:8080
```

The output shows the directory structure and file types:

```
/
├── bin/
├── etc/
│   ├── passwd
│   └── hosts
└── usr/
    └── bin/
```

### Inspecting Flist Details

You can inspect the details of an entire flist, showing metadata for all files and directories:

```bash
# Inspect a local flist
rfs flist inspect output.fl

# Inspect a flist from a server using its hash
rfs flist inspect abc123... --server-url http://localhost:8080
```

The output includes detailed information for each file and directory in the flist:

```
Path: /etc/passwd
  Type: Regular File
  Inode: 12345
  Name: passwd
  Size: 1234 bytes
  UID: 0
  GID: 0
  Mode: 0100644
  Permissions: 0644
  Device: 0
  Created: 1609459200
  Modified: 1609459200
  ---
```

At the end, a summary is displayed:

```
Flist Inspection: output.fl
==================
Files: 236
Directories: 42
Symlinks: 15
Total size: 5834752 bytes
```

This is particularly useful for verifying the contents of an flist before distributing it or for troubleshooting issues.

## Next Steps

Now that you know how to create flists, you might want to learn:

- [Converting Docker Images to Flists](./docker-conversion.md)
- [Mounting and Using Flists](./mounting-flists.md)
- [Setting Up the RFS Server](./server-setup.md)

For more detailed information about flists and storage backends, see the [Concepts](../concepts/) documentation.