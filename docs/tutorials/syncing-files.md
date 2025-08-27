# Syncing Files Between RFS Servers

This tutorial will guide you through the process of syncing files and blocks between RFS servers. You'll learn how to set up multiple servers, sync individual files, and synchronize entire directories.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- At least two RFS servers running (see the [Server Setup](./server-setup.md) tutorial)
- Basic understanding of command-line operations

## Understanding RFS Syncing

RFS allows you to sync files and blocks between servers, which is useful for:

- **Replication**: Ensuring content is available on multiple servers for redundancy
- **Distribution**: Spreading content across multiple servers for load balancing
- **Migration**: Moving content from one server to another
- **Backup**: Creating backups of important content on separate servers

The sync process works by:

1. Checking if a file or block exists on the destination server
2. If it doesn't exist, copying it from the source server to the destination server
3. Verifying the integrity of the transferred data

## Step 1: Set Up Two RFS Servers

For this tutorial, we'll assume you have two RFS servers running:

- Source server: `http://localhost:3000`
- Destination server: `http://localhost:3001`

If you don't have two servers running, you can set them up following the [Server Setup](./server-setup.md) tutorial, using different port numbers.

## Step 2: Upload a File to the Source Server

Let's upload a file to the source server:

```bash
# Create a sample file
echo "This is a sample file for syncing" > ~/sample-file.txt

# Upload the file to the source server
rfs upload ~/sample-file.txt --server http://localhost:3000
```

You should see output similar to:

```
Uploading file: /home/user/sample-file.txt
File uploaded successfully!
File hash: abc123...
```

Note the file hash from the output. We'll use this to sync the file to the destination server.

## Step 3: Sync the File to the Destination Server

Now, let's sync the file to the destination server:

```bash
# Sync the file using its hash
rfs sync --hash abc123... --source http://localhost:3000 --destination http://localhost:3001
```

You should see output similar to:

```
Syncing hash: abc123...
Checking if hash exists on destination server...
Hash does not exist on destination server.
Downloading from source server...
Uploading to destination server...
Sync completed successfully!
```

## Step 4: Verify the Sync

Let's verify that the file is now available on the destination server:

```bash
# Check if the file exists on the destination server
rfs exists abc123... --server http://localhost:3001
```

You should see output confirming that the file exists on the destination server.

## Step 5: Upload a Directory to the Source Server

Now, let's upload a directory to the source server:

```bash
# Create a sample directory with multiple files
mkdir -p ~/sample-dir/{subdir1,subdir2}
echo "File 1 content" > ~/sample-dir/file1.txt
echo "File 2 content" > ~/sample-dir/file2.txt
echo "Subdir file 1" > ~/sample-dir/subdir1/file1.txt
echo "Subdir file 2" > ~/sample-dir/subdir2/file2.txt

# Upload the directory to the source server
rfs upload-dir ~/sample-dir --server http://localhost:3000 --create-flist
```

You should see output similar to:

```
Uploading directory: /home/user/sample-dir
Found 4 files, 3 directories
Processed 4 files, 48 bytes total
Created 4 unique blocks, 48 bytes total
Directory uploaded successfully!
Flist created with hash: def456...
```

Note the flist hash from the output. We'll use this to sync the directory to the destination server.

## Step 6: Sync the Directory to the Destination Server

Now, let's sync the directory to the destination server:

```bash
# Sync the directory using its flist hash
rfs sync --hash def456... --source http://localhost:3000 --destination http://localhost:3001
```

You should see output similar to:

```
Syncing hash: def456...
Checking if hash exists on destination server...
Hash does not exist on destination server.
Downloading from source server...
Processing flist...
Syncing 4 blocks...
Uploading to destination server...
Sync completed successfully!
```

## Step 7: Download the Directory from the Destination Server

To verify that the directory was synced correctly, let's download it from the destination server:

```bash
# Create a directory for the download
mkdir -p ~/downloaded-dir

# Download the directory from the destination server
rfs download-dir def456... --output ~/downloaded-dir --server http://localhost:3001
```

You should see output similar to:

```
Downloading directory with flist hash: def456...
Downloading flist...
Processing flist...
Downloading 4 files...
Directory downloaded successfully to: /home/user/downloaded-dir
```

Now you can compare the original directory with the downloaded directory:

```bash
# Compare the directories
diff -r ~/sample-dir ~/downloaded-dir
```

If there's no output, the directories are identical, confirming that the sync was successful.

## Advanced: Automated Syncing

For production use, you might want to set up automated syncing between servers. This can be done using cron jobs or systemd timers.

### Using Cron

```bash
# Edit the crontab
crontab -e

# Add a line to sync every hour
0 * * * * /usr/local/bin/rfs sync --hash abc123... --source http://server1:3000 --destination http://server2:3000
```

### Using Systemd Timer

Create a service file `/etc/systemd/system/rfs-sync.service`:

```ini
[Unit]
Description=RFS Sync Service
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/rfs sync --hash abc123... --source http://server1:3000 --destination http://server2:3000

[Install]
WantedBy=multi-user.target
```

Create a timer file `/etc/systemd/system/rfs-sync.timer`:

```ini
[Unit]
Description=Run RFS Sync every hour

[Timer]
OnBootSec=15min
OnUnitActiveSec=1h

[Install]
WantedBy=timers.target
```

Enable and start the timer:

```bash
sudo systemctl enable rfs-sync.timer
sudo systemctl start rfs-sync.timer
```

## Advanced: Syncing Multiple Hashes

If you need to sync multiple files or directories, you can create a script:

```bash
#!/bin/bash

# List of hashes to sync
HASHES=("abc123..." "def456..." "ghi789...")

# Source and destination servers
SOURCE="http://server1:3000"
DESTINATION="http://server2:3000"

# Sync each hash
for HASH in "${HASHES[@]}"; do
  echo "Syncing hash: $HASH"
  rfs sync --hash "$HASH" --source "$SOURCE" --destination "$DESTINATION"
done
```

## Troubleshooting

### Sync Fails with Connection Error

If the sync fails with a connection error:

1. Check that both servers are running
2. Verify the server URLs are correct
3. Check for network connectivity issues
4. Ensure there are no firewalls blocking the connection

### Hash Not Found on Source Server

If the hash is not found on the source server:

1. Verify the hash is correct
2. Check if the file was deleted from the source server
3. Try uploading the file again

### Insufficient Disk Space

If the sync fails due to insufficient disk space:

1. Check the available disk space on the destination server
2. Free up space if necessary
3. Consider using a different storage backend with more capacity

## Next Steps

Now that you've learned how to sync files between RFS servers, you might want to:

- Learn about [website publishing](./website-publishing.md)
- Explore [advanced server configuration](../user-guides/fl-server.md)
- Set up [multiple storage backends](../concepts/stores.md) for redundancy
- Learn about [sharding and replication](../concepts/sharding.md) for distributed storage