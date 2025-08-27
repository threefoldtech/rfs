# Understanding Caching in RFS

This document explains how caching works in the RFS ecosystem, its importance, and how to optimize it for different use cases.

## What is Caching in RFS?

In RFS, caching refers to the temporary local storage of file content that has been retrieved from remote storage backends. When a file is accessed from a mounted flist, RFS downloads the necessary content chunks from the storage backend and stores them in a local cache directory for faster subsequent access.

Caching is a critical component of RFS that balances performance with resource efficiency.

## How Caching Works

### The Caching Process

1. **Initial Access**: When a file in a mounted flist is accessed for the first time, RFS:
   - Identifies the chunks needed based on the flist metadata
   - Retrieves the chunks from the storage backend
   - Stores the chunks in the local cache directory
   - Assembles the file content from the cached chunks

2. **Subsequent Access**: When the same file is accessed again, RFS:
   - Checks if the required chunks are in the cache
   - Uses the cached chunks if available
   - Only retrieves missing chunks from the storage backend

3. **Cache Organization**: The cache directory is organized by content hashes, with each chunk stored as a separate file named after its hash.

### Cache Location

By default, RFS uses `/tmp/cache` as the cache directory. This can be customized using the `-c` or `--cache` option:

```bash
# Mount an flist with a custom cache directory
sudo rfs mount -m flist.fl -c /path/to/cache /mount/point
```

### Cache Persistence

The cache persists between mounts and even system reboots (unless `/tmp` is cleared). This means that if you mount an flist, access some files, unmount it, and then mount it again, the previously accessed files will be served from the cache.

## Benefits of Caching

### 1. Performance Improvement

Caching significantly improves performance by:
- **Reducing Network Latency**: Cached content is accessed locally, eliminating network delays
- **Reducing Bandwidth Usage**: Content is only downloaded once, even if accessed multiple times
- **Improving Responsiveness**: Applications using mounted flists feel more responsive

### 2. Offline Access

Once content is cached, it can be accessed even if the storage backend is temporarily unavailable, providing a degree of offline functionality.

### 3. Reduced Load on Storage Backends

Caching reduces the load on storage backends by minimizing the number of requests, which is particularly important for shared or public storage backends.

## Cache Management

### Cache Size

The cache can grow over time as more files are accessed. It's important to monitor and manage cache size, especially on systems with limited storage.

There is no built-in automatic cache size limitation or cleanup mechanism, so manual management may be necessary for long-running systems.

### Clearing the Cache

To clear the cache, simply remove the contents of the cache directory:

```bash
# Clear the default cache
rm -rf /tmp/cache/*

# Clear a custom cache
rm -rf /path/to/cache/*
```

### Pre-warming the Cache

In some cases, you might want to "pre-warm" the cache by accessing files before they're needed:

```bash
# Mount the flist
sudo rfs mount -m flist.fl -c /path/to/cache /mount/point

# Access all files to cache them
find /mount/point -type f -exec cat {} > /dev/null \;
```

This can be useful for ensuring responsive performance when the files are actually needed.

## Parallel Downloads

RFS uses parallel downloads to improve performance when retrieving multiple chunks simultaneously. This is particularly beneficial for:

- **Large Files**: Files split into multiple chunks
- **Multiple Files**: When accessing multiple files at once
- **High-Latency Networks**: Where the round-trip time is significant

The number of parallel downloads is automatically determined based on system resources and network conditions.

## Cache Considerations for Different Use Cases

### Development and Testing

For development and testing, the default cache settings are usually sufficient:

```bash
sudo rfs mount -m dev.fl /mount/point
```

### Production Deployments

For production deployments, consider:

1. **Persistent Cache Location**: Use a persistent location for the cache
   ```bash
   sudo rfs mount -m prod.fl -c /var/cache/rfs /mount/point
   ```

2. **Sufficient Cache Space**: Ensure the cache location has enough space
   ```bash
   # Check available space
   df -h /var/cache/rfs
   ```

3. **Cache Monitoring**: Set up monitoring for cache size and usage
   ```bash
   # Example monitoring script
   du -sh /var/cache/rfs
   ```

### High-Performance Requirements

For high-performance requirements:

1. **SSD Storage**: Place the cache on SSD storage for faster access
   ```bash
   sudo rfs mount -m highperf.fl -c /mnt/ssd/cache /mount/point
   ```

2. **Pre-warming**: Pre-warm the cache for critical files
   ```bash
   # Pre-warm specific directories
   find /mount/point/critical/path -type f -exec cat {} > /dev/null \;
   ```

3. **Dedicated Cache**: Use a dedicated cache for each flist to prevent contention
   ```bash
   sudo rfs mount -m app1.fl -c /var/cache/rfs/app1 /mount/app1
   sudo rfs mount -m app2.fl -c /var/cache/rfs/app2 /mount/app2
   ```

### Limited Storage Environments

For environments with limited storage:

1. **Regular Cleanup**: Implement regular cache cleanup
   ```bash
   # Cleanup script example
   find /var/cache/rfs -type f -atime +30 -delete
   ```

2. **Selective Caching**: Only cache frequently accessed files
   ```bash
   # Mount without accessing all files
   sudo rfs mount -m limited.fl -c /var/cache/rfs-limited /mount/point
   ```

3. **Compressed Filesystem**: Consider using a compressed filesystem for the cache
   ```bash
   # Create a compressed filesystem
   sudo mkfs.btrfs -L rfs-cache /dev/sdX
   sudo mount -o compress=zstd /dev/sdX /var/cache/rfs
   ```

## Cache Consistency and Integrity

### Content Verification

RFS verifies the integrity of cached content using cryptographic hashes. If a cached chunk is corrupted, RFS will detect the mismatch and retrieve the chunk again from the storage backend.

### Cache Invalidation

RFS does not currently implement automatic cache invalidation. If the content in the storage backend changes, the cached content will not be automatically updated.

To force a refresh of cached content:

```bash
# Clear the cache
rm -rf /path/to/cache/*

# Remount the flist
sudo umount /mount/point
sudo rfs mount -m updated.fl -c /path/to/cache /mount/point
```

## Troubleshooting Cache Issues

### Insufficient Cache Space

If the cache directory runs out of space:

1. **Check Available Space**:
   ```bash
   df -h /path/to/cache
   ```

2. **Clear Unused Cache**:
   ```bash
   # Remove old cache entries
   find /path/to/cache -type f -atime +7 -delete
   ```

3. **Allocate More Space**:
   ```bash
   # Move cache to a larger partition
   sudo mkdir -p /mnt/large/cache
   sudo mount -o bind /mnt/large/cache /path/to/cache
   ```

### Corrupted Cache

If you suspect cache corruption:

1. **Clear the Entire Cache**:
   ```bash
   rm -rf /path/to/cache/*
   ```

2. **Verify Storage Backend Access**:
   ```bash
   # For HTTP store
   curl -I http://store.example.com/some/path
   ```

3. **Check for Disk Errors**:
   ```bash
   sudo fsck -f /dev/sdX
   ```

### Performance Issues

If cache performance is poor:

1. **Check Disk I/O**:
   ```bash
   iostat -x 1
   ```

2. **Check for Fragmentation**:
   ```bash
   sudo filefrag -v /path/to/cache/some-file
   ```

3. **Consider a Faster Storage Medium**:
   ```bash
   # Move cache to SSD
   sudo mkdir -p /mnt/ssd/cache
   sudo rsync -av /path/to/cache/ /mnt/ssd/cache/
   ```

## Best Practices

1. **Dedicated Cache Location**: Use a dedicated location for the cache, separate from temporary directories that might be cleaned up automatically.

2. **Sufficient Space**: Ensure the cache location has enough space for your expected usage.

3. **Regular Monitoring**: Monitor cache size and growth over time.

4. **Periodic Cleanup**: Implement periodic cleanup for long-running systems.

5. **Performance Optimization**: Place the cache on fast storage for performance-critical applications.

6. **Backup Consideration**: Remember that the cache contains duplicated data and typically doesn't need to be included in backups.

## Next Steps

For more information about related concepts, see:
- [Understanding Flists](./flists.md)
- [Understanding Storage Backends](./stores.md)
- [Understanding Sharding](./sharding.md)

For practical guides on working with RFS, see the [Tutorials](../tutorials/) section.