# Performance Tuning Guide

This guide provides recommendations and best practices for optimizing the performance of RFS components in various deployment scenarios.

## Introduction

RFS performance can be affected by various factors, including hardware resources, network conditions, storage backend configuration, and usage patterns. This guide will help you identify and address performance bottlenecks in your RFS deployment.

## System Requirements

### Minimum Requirements

- **CPU**: 2 cores
- **RAM**: 2 GB
- **Disk**: 10 GB (plus storage for content)
- **Network**: 10 Mbps

### Recommended Requirements

- **CPU**: 4+ cores
- **RAM**: 8+ GB
- **Disk**: SSD storage for cache and metadata
- **Network**: 100+ Mbps

## RFS Core Performance Tuning

### Parallel Downloads

RFS uses parallel downloads to improve performance when retrieving content from storage backends. You can control the number of parallel downloads using the `RFS_PARALLEL_DOWNLOAD` environment variable:

```bash
# Set to a specific number (e.g., 8)
export RFS_PARALLEL_DOWNLOAD=8

# Run RFS command
rfs unpack -m flist.fl ~/extracted
```

The optimal value depends on your system resources and network conditions:

- **CPU-limited systems**: Lower values (4-8)
- **Network-limited systems**: Higher values (16-32)
- **Balanced systems**: Moderate values (8-16)

If not specified, RFS automatically determines an appropriate value based on system resources.

### Cache Configuration

The cache is critical for performance. Consider these optimizations:

1. **Cache Location**: Place the cache on fast storage (SSD)
   ```bash
   rfs mount -m flist.fl -c /mnt/ssd/cache /mount/point
   ```

2. **Cache Size**: Ensure sufficient space for your working set
   ```bash
   # Check available space
   df -h /path/to/cache
   ```

3. **Cache Persistence**: Use a persistent location for long-running mounts
   ```bash
   rfs mount -m flist.fl -c /var/cache/rfs /mount/point
   ```

4. **Pre-warming**: Access frequently used files to cache them
   ```bash
   find /mount/point/important/path -type f -exec cat {} > /dev/null \;
   ```

### Storage Backend Selection

Choose appropriate storage backends for your use case:

1. **Local Development**: Use directory stores
   ```bash
   rfs pack -m output.fl -s dir:///tmp/store /path/to/directory
   ```

2. **Production**: Use ZDB or S3 stores
   ```bash
   rfs pack -m output.fl -s zdb://zdb.example.com:9900/namespace /path/to/directory
   ```

3. **High Performance**: Use local SSD-backed stores
   ```bash
   rfs pack -m output.fl -s dir:///mnt/ssd/store /path/to/directory
   ```

### Sharding and Replication

Use sharding and replication to optimize performance:

1. **Sharding for Parallel Access**: Distribute content across multiple backends
   ```bash
   rfs pack -m output.fl \
     -s 00-3f=dir:///tmp/store1 \
     -s 40-7f=dir:///tmp/store2 \
     -s 80-bf=dir:///tmp/store3 \
     -s c0-ff=dir:///tmp/store4 \
     /path/to/directory
   ```

2. **Replication for Availability**: Replicate content for redundancy
   ```bash
   rfs pack -m output.fl \
     -s dir:///tmp/store1 \
     -s dir:///tmp/store2 \
     /path/to/directory
   ```

3. **Geographic Distribution**: Place content close to users
   ```bash
   rfs pack -m output.fl \
     -s 00-7f=s3://us-east-1.amazonaws.com/us-bucket \
     -s 80-ff=s3://eu-west-1.amazonaws.com/eu-bucket \
     /path/to/directory
   ```

## FL Server Performance Tuning

### Server Hardware

The FL server benefits from:

- **Multiple CPU cores**: For handling concurrent requests
- **Sufficient RAM**: For caching and processing
- **Fast storage**: For flist storage and temporary files
- **Network bandwidth**: For transferring flists and content

### Concurrent Requests

The FL server can handle multiple concurrent requests. To optimize for high concurrency:

1. **Increase system limits**: Adjust file descriptor limits
   ```bash
   # Check current limits
   ulimit -n

   # Set higher limits in /etc/security/limits.conf
   # username soft nofile 65536
   # username hard nofile 65536
   ```

2. **Configure reverse proxy**: If using Nginx or similar, optimize for concurrency
   ```nginx
   # Nginx example
   worker_processes auto;
   worker_connections 4096;
   ```

### Storage Configuration

Optimize the storage configuration for the FL server:

1. **Fast local storage**: Use SSD for the `flist_dir`
   ```toml
   flist_dir = "/mnt/ssd/flists"
   ```

2. **Efficient store URLs**: Use performant storage backends
   ```toml
   store_url = ["dir:///mnt/ssd/store"]
   ```

3. **Multiple stores**: Use sharding for better performance
   ```toml
   store_url = [
       "00-7f=dir:///mnt/ssd1/store",
       "80-ff=dir:///mnt/ssd2/store"
   ]
   ```

## Frontend Performance Tuning

### Build Optimization

For production deployment, build the frontend with optimization:

```bash
npm run build
```

This creates optimized assets in the `dist` directory.

### Serving Static Assets

Use efficient methods to serve the frontend:

1. **Content Delivery Network (CDN)**: Deploy static assets to a CDN
2. **HTTP/2**: Enable HTTP/2 in your web server
3. **Compression**: Enable gzip or Brotli compression
4. **Caching**: Configure appropriate cache headers

Example Nginx configuration:

```nginx
server {
    listen 443 ssl http2;
    server_name your-domain.example.com;

    root /path/to/rfs/frontend/dist;
    index index.html;

    gzip on;
    gzip_types text/plain text/css application/javascript application/json;

    location /assets/ {
        expires 1y;
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

## Network Optimization

### Bandwidth Management

1. **Quality of Service (QoS)**: Prioritize RFS traffic if needed
2. **Traffic Shaping**: Limit bandwidth usage during peak times
3. **Compression**: Enable compression for HTTP-based stores

### Latency Reduction

1. **Geographic Proximity**: Use storage backends close to users
2. **Connection Pooling**: Maintain persistent connections to backends
3. **DNS Optimization**: Ensure fast DNS resolution for backend hostnames

## Monitoring and Profiling

### System Monitoring

Monitor system resources to identify bottlenecks:

```bash
# CPU and memory usage
top

# Disk I/O
iostat -x 1

# Network usage
iftop
```

### RFS Profiling

Enable debug logging to identify performance issues:

```bash
# Enable debug logging
rfs --debug mount -m flist.fl /mount/point
```

### FL Server Monitoring

Monitor the FL server using standard tools:

```bash
# Process monitoring
ps aux | grep "rfs server"

# Log monitoring
tail -f rfs-server.log
```

## Performance Testing

### Benchmarking Tools

Use these tools to benchmark RFS performance:

1. **fio**: For disk I/O benchmarking
   ```bash
   fio --name=test --filename=/mount/point/testfile --direct=1 --rw=randread --bs=4k --size=1G --numjobs=4 --runtime=60 --group_reporting
   ```

2. **dd**: For simple throughput testing
   ```bash
   dd if=/mount/point/largefile of=/dev/null bs=1M count=1000
   ```

3. **time**: For measuring command execution time
   ```bash
   time rfs unpack -m flist.fl ~/extracted
   ```

### Performance Metrics

Key metrics to monitor:

1. **Mount Time**: Time to mount an flist
2. **First Access Time**: Time to first access a file (cold cache)
3. **Subsequent Access Time**: Time to access a file again (warm cache)
4. **Throughput**: Data transfer rate for large files
5. **IOPS**: Operations per second for small files

## Common Performance Issues and Solutions

### Slow Mount Operations

**Symptoms:**
- Long delay when mounting flists
- High CPU usage during mount

**Solutions:**
- Use smaller flists with fewer files
- Ensure fast metadata access (SSD storage)
- Pre-warm the cache for frequently accessed flists

### Slow First Access

**Symptoms:**
- Long delay when accessing a file for the first time
- Normal performance for subsequent access

**Solutions:**
- Use storage backends with lower latency
- Increase parallel download count
- Pre-warm the cache for important files

### Network Bottlenecks

**Symptoms:**
- Good performance with local stores, poor with remote stores
- Network saturation during file access

**Solutions:**
- Use more efficient storage backends
- Implement sharding across multiple backends
- Consider local caching proxies for remote backends

### High CPU Usage

**Symptoms:**
- CPU saturation during RFS operations
- Slow performance despite fast storage and network

**Solutions:**
- Reduce parallel download count
- Use more efficient storage backends
- Distribute load across multiple instances

### Memory Limitations

**Symptoms:**
- Increasing memory usage over time
- Swapping during heavy usage

**Solutions:**
- Limit parallel operations
- Ensure sufficient RAM for your workload
- Consider memory-optimized instances for large deployments

## Advanced Performance Techniques

### Custom Kernel Parameters

Tune kernel parameters for better performance:

```bash
# Increase the maximum number of open files
sysctl -w fs.file-max=1000000

# Increase the maximum number of inotify watches
sysctl -w fs.inotify.max_user_watches=524288

# Optimize network parameters
sysctl -w net.core.somaxconn=4096
sysctl -w net.ipv4.tcp_max_syn_backlog=4096
```

### FUSE Optimization

Optimize FUSE mount options:

```bash
# Mount with optimized FUSE options
rfs mount -m flist.fl -o big_writes,max_read=131072,max_write=131072 /mount/point
```

### Custom Compilation

Compile RFS with performance optimizations:

```bash
# Build with optimizations
RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release
```

## Conclusion

Performance tuning is an iterative process. Start with the recommendations in this guide, monitor performance, identify bottlenecks, and make targeted improvements.

Remember that the optimal configuration depends on your specific use case, hardware, and network environment. What works best for one deployment may not be ideal for another.

## Next Steps

For more information about related topics, see:
- [RFS CLI User Guide](./rfs-cli.md)
- [RFS Server User Guide](./fl-server.md)
- [Understanding Sharding and Replication](../concepts/sharding.md)