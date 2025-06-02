# Mounting and Using Flists

This tutorial will guide you through the process of mounting flists as filesystems and working with the mounted content. Mounting an flist allows you to access its contents as if they were regular files on your system.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- FUSE installed on your system
- Root/sudo access (required for mounting)
- An flist file (created as described in [Creating Flists](./creating-flists.md) or [Docker Conversion](./docker-conversion.md))

## Understanding Flist Mounting

When you mount an flist, RFS creates a FUSE (Filesystem in Userspace) mount point that presents the flist's contents as a regular filesystem. Key points to understand:

- The mount is **read-only** by default
- Files are downloaded from the storage backend **on-demand** when accessed
- Downloaded content is **cached** locally for improved performance
- The mount requires **root privileges** due to FUSE security restrictions

## Basic Mounting

### Mounting an Flist

To mount an flist, use the `mount` subcommand:

```bash
# Create a mount point
mkdir -p ~/mount-point

# Mount the flist
sudo rfs mount -m path/to/your.fl -c ~/rfs-cache ~/mount-point
```

This command:
- Mounts the flist `path/to/your.fl` at `~/mount-point`
- Uses `~/rfs-cache` as a cache directory for downloaded content
- Runs in the foreground (the terminal will be occupied until unmounting)

### Accessing the Mounted Content

Once the flist is mounted, you can access its contents like any other filesystem:

```bash
# List the contents of the mount point
ls -la ~/mount-point

# Navigate into directories
cd ~/mount-point/some/directory

# View file contents
cat ~/mount-point/path/to/file.txt

# Copy files from the mount
cp ~/mount-point/path/to/file.txt ~/destination/
```

### Unmounting the Flist

To unmount the flist, use the standard `umount` command:

```bash
sudo umount ~/mount-point
```

If you're running the mount command in the foreground, you can also press `Ctrl+C` to terminate the process and unmount the filesystem.

## Advanced Mounting

### Running as a Daemon

For long-term mounts, you can run RFS in daemon mode:

```bash
sudo rfs mount -m path/to/your.fl -c ~/rfs-cache --daemon --log ~/rfs-mount.log ~/mount-point
```

This command:
- Runs the mount process in the background
- Logs output to `~/rfs-mount.log`
- Allows you to continue using the terminal

To unmount a daemon-mode mount, use the standard `umount` command:

```bash
sudo umount ~/mount-point
```

### Customizing Cache Location

The cache directory stores downloaded content for improved performance. You can customize its location:

```bash
sudo rfs mount -m path/to/your.fl -c /custom/cache/path ~/mount-point
```

Consider these factors when choosing a cache location:
- **Disk space**: Ensure sufficient space for cached content
- **Performance**: Using an SSD can improve access speed
- **Persistence**: Cache contents persist between mounts unless cleared

### Mounting Multiple Flists

You can mount multiple flists simultaneously by using different mount points:

```bash
# Mount the first flist
sudo rfs mount -m first.fl -c ~/cache/first --daemon --log ~/log/first.log ~/mount/first

# Mount the second flist
sudo rfs mount -m second.fl -c ~/cache/second --daemon --log ~/log/second.log ~/mount/second
```

## Working with Mounted Flists

### Running Applications from Mounted Flists

If your flist contains executable files, you can run them directly:

```bash
# Run an executable from the mount
~/mount-point/path/to/executable
```

### Using chroot with Mounted Flists

For Docker-converted flists or complete system flists, you can use `chroot` to run commands within the flist's environment:

```bash
# Run a shell in the flist environment
sudo chroot ~/mount-point /bin/sh

# Run a specific command in the flist environment
sudo chroot ~/mount-point /bin/sh -c "ls -la /usr/bin"
```

### Copying Content from Mounted Flists

To extract content from a mounted flist to your local filesystem:

```bash
# Copy a single file
cp ~/mount-point/path/to/file.txt ~/destination/

# Copy a directory recursively
cp -r ~/mount-point/path/to/directory ~/destination/

# Use rsync for more control
rsync -av ~/mount-point/path/to/directory/ ~/destination/
```

## Performance Considerations

### Cache Management

The cache improves performance by storing downloaded content locally:

```bash
# Clear the cache to free up space
rm -rf ~/rfs-cache/*

# Pre-warm the cache by accessing files
find ~/mount-point -type f -exec cat {} > /dev/null \;
```

### Parallel Downloads

RFS uses parallel downloads to improve performance when accessing multiple files. This behavior is automatic and doesn't require configuration.

### Network Performance

When using remote storage backends, network performance affects file access speed:

- **Latency**: High latency can slow down initial file access
- **Bandwidth**: Limited bandwidth can reduce transfer speeds
- **Reliability**: Network interruptions can cause access failures

## Troubleshooting

### Mount Failures

If mounting fails, check:

1. **FUSE Installation**: Ensure FUSE is properly installed
   ```bash
   # Check if FUSE is installed
   ls -l /dev/fuse
   ```

2. **Permissions**: Ensure you're using sudo or have proper permissions
   ```bash
   # Check if you're in the fuse group
   groups | grep fuse
   ```

3. **Mount Point**: Ensure the mount point exists and is empty
   ```bash
   # Create a fresh mount point
   mkdir -p ~/new-mount-point
   ```

### Access Errors

If you encounter errors accessing files:

1. **Storage Backend**: Ensure the storage backend is accessible
   ```bash
   # Check if you can access the store directly
   curl -I http://your-store-url/some-path
   ```

2. **Cache Directory**: Ensure the cache directory is writable
   ```bash
   # Check cache directory permissions
   ls -ld ~/rfs-cache
   ```

3. **Flist Integrity**: Verify the flist is valid
   ```bash
   # Check flist metadata
   rfs config -m path/to/your.fl tag list
   ```

### Unmount Issues

If unmounting fails:

1. **Busy Mount**: Ensure no processes are using the mount
   ```bash
   # Find processes using the mount
   lsof | grep mount-point
   ```

2. **Forced Unmount**: Use the force option if necessary
   ```bash
   # Force unmount
   sudo umount -f ~/mount-point
   ```

## Examples

### Mounting a Web Application Flist

```bash
# Mount a web application flist
sudo rfs mount -m webapp.fl -c ~/cache/webapp ~/mount/webapp

# Serve the web application using a simple HTTP server
cd ~/mount/webapp
python3 -m http.server 8080
```

### Mounting a Docker-Converted Flist

```bash
# Mount a Docker-converted flist
sudo rfs mount -m nginx-latest.fl -c ~/cache/nginx ~/mount/nginx

# Run a command in the nginx environment
sudo chroot ~/mount/nginx /usr/sbin/nginx -t
```

### Mounting a Development Environment Flist

```bash
# Mount a development environment flist
sudo rfs mount -m dev-env.fl -c ~/cache/dev-env ~/mount/dev-env

# Use the development environment
sudo chroot ~/mount/dev-env /bin/bash
```

## Next Steps

Now that you know how to mount and use flists, you might want to learn:

- [Setting Up the FL Server](./server-setup.md)
- [Using the FL Server Web Interface](../user-guides/frontend.md)
- [Performance Tuning](../user-guides/performance-tuning.md)

For more detailed information about mounting options, see the [RFS CLI User Guide](../user-guides/rfs-cli.md).