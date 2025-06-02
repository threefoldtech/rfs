# Troubleshooting Guide

This guide provides solutions for common issues you might encounter when using RFS and its components.

## RFS Core Issues

### Mount Failures

#### Issue: Cannot mount flist

**Symptoms:**
- Error message: `Failed to mount flist`
- FUSE-related errors

**Possible Causes:**
1. Missing FUSE installation
2. Insufficient permissions
3. Mount point issues
4. Invalid flist file

**Solutions:**

1. **Check FUSE Installation**
   ```bash
   # Check if FUSE is installed
   ls -l /dev/fuse
   
   # Install FUSE if missing
   # Ubuntu/Debian
   sudo apt-get install fuse libfuse-dev
   
   # Fedora/CentOS/RHEL
   sudo dnf install fuse fuse-devel
   ```

2. **Check Permissions**
   ```bash
   # Ensure you're using sudo
   sudo rfs mount -m flist.fl /mount/point
   
   # Check if you're in the fuse group
   groups | grep fuse
   
   # Add yourself to the fuse group if needed
   sudo usermod -a -G fuse $USER
   ```

3. **Check Mount Point**
   ```bash
   # Ensure the mount point exists
   mkdir -p /mount/point
   
   # Ensure the mount point is empty
   ls -la /mount/point
   
   # Ensure the mount point is not already mounted
   mountpoint /mount/point
   ```

4. **Verify Flist File**
   ```bash
   # Check if the flist file exists
   ls -la flist.fl
   
   # Check flist metadata
   rfs config -m flist.fl tag list
   ```

#### Issue: Mount is slow or unresponsive

**Symptoms:**
- Long delay when mounting flists
- Mounted filesystem is slow to respond
- High CPU or memory usage

**Solutions:**

1. **Check Storage Backend Connectivity**
   ```bash
   # For HTTP store
   curl -I http://store.example.com/path
   
   # For ZDB store
   telnet zdb.example.com 9900
   ```

2. **Optimize Cache Location**
   ```bash
   # Use SSD for cache
   rfs mount -m flist.fl -c /mnt/ssd/cache /mount/point
   ```

3. **Check System Resources**
   ```bash
   # Monitor CPU and memory
   top
   
   # Check disk I/O
   iostat -x 1
   ```

4. **Enable Debug Logging**
   ```bash
   # Run with debug logging
   sudo rfs --debug mount -m flist.fl /mount/point
   ```

### Storage Backend Issues

#### Issue: Cannot connect to storage backend

**Symptoms:**
- Error message: `Failed to connect to store`
- Timeout errors
- Network-related errors

**Solutions:**

1. **Check Network Connectivity**
   ```bash
   # Check if the host is reachable
   ping store.example.com
   
   # Check if the port is open
   telnet store.example.com 9900
   ```

2. **Verify Store URL**
   ```bash
   # List stores in the flist
   rfs config -m flist.fl store list
   
   # Try adding an alternative store
   rfs config -m flist.fl store add -s alternative-store-url
   ```

3. **Check Credentials**
   ```bash
   # For S3 store
   aws s3 ls s3://bucket-name --endpoint-url http://s3.example.com:9000
   ```

4. **Check Firewall Rules**
   ```bash
   # Check if outgoing connections are allowed
   sudo iptables -L -n
   ```

#### Issue: Content not found in storage backend

**Symptoms:**
- Error message: `Content not found`
- Files appear in directory listing but cannot be accessed

**Solutions:**

1. **Verify Content Exists**
   ```bash
   # For directory store
   find /tmp/store -type f | wc -l
   
   # For HTTP store
   curl -I http://store.example.com/some/path
   ```

2. **Check Sharding Configuration**
   ```bash
   # List stores in the flist
   rfs config -m flist.fl store list
   ```

3. **Try Adding Alternative Stores**
   ```bash
   # Add a backup store
   rfs config -m flist.fl store add -s backup-store-url
   ```

4. **Recreate the Flist**
   ```bash
   # If possible, recreate the flist
   rfs pack -m new.fl -s store-url /path/to/directory
   ```

### Unpack Issues

#### Issue: Cannot unpack flist

**Symptoms:**
- Error message: `Failed to unpack flist`
- Permission errors
- Storage errors

**Solutions:**

1. **Check Destination Permissions**
   ```bash
   # Ensure you have write permission
   ls -ld /destination/directory
   
   # Change permissions if needed
   chmod 755 /destination/directory
   ```

2. **Check Available Space**
   ```bash
   # Ensure sufficient disk space
   df -h /destination/directory
   ```

3. **Check Storage Backend Connectivity**
   ```bash
   # Verify storage backend is accessible
   # (See solutions for storage backend issues)
   ```

4. **Try Without Preserving Ownership**
   ```bash
   # Unpack without preserving ownership
   rfs unpack -m flist.fl /destination/directory
   ```

#### Issue: Unpacked files have wrong permissions

**Symptoms:**
- Files have unexpected ownership or permissions
- Cannot access unpacked files

**Solutions:**

1. **Use Preserve Ownership Flag**
   ```bash
   # Unpack with preserved ownership (requires sudo)
   sudo rfs unpack -m flist.fl -p /destination/directory
   ```

2. **Check Original File Permissions**
   ```bash
   # Mount the flist and check permissions
   sudo rfs mount -m flist.fl /mount/point
   ls -la /mount/point
   ```

3. **Fix Permissions After Unpacking**
   ```bash
   # Change ownership
   sudo chown -R user:group /destination/directory
   
   # Fix permissions
   sudo chmod -R u+rw /destination/directory
   ```

## FL Server Issues

### Server Startup Issues

#### Issue: Server won't start

**Symptoms:**
- Error message: `Failed to start server`
- Port binding errors
- Configuration errors

**Solutions:**

1. **Check Configuration File**
   ```bash
   # Validate TOML syntax
   cat config.toml | python3 -c "import tomli; import sys; tomli.loads(sys.stdin.read())"
   ```

2. **Check Port Availability**
   ```bash
   # Check if the port is already in use
   netstat -tuln | grep 3000
   
   # Kill process using the port if needed
   sudo fuser -k 3000/tcp
   ```

3. **Check Directory Permissions**
   ```bash
   # Ensure flist_dir exists and is writable
   ls -ld flists
   
   # Create directory if missing
   mkdir -p flists
   ```

4. **Run with Debug Logging**
   ```bash
   # Start with debug flag
   rfs server --config-path config.toml --debug
   ```

### Authentication Issues

#### Issue: Cannot authenticate

**Symptoms:**
- Error message: `Authentication failed`
- Invalid token errors
- Login failures

**Solutions:**

1. **Verify User Configuration**
   ```bash
   # Check user configuration in config.toml
   grep -A 2 "[[users]]" config.toml
   ```

2. **Check JWT Secret**
   ```bash
   # Ensure jwt_secret is set in config.toml
   grep "jwt_secret" config.toml
   ```

3. **Check Token Expiration**
   ```bash
   # Set longer token expiration
   # jwt_expire_hours = 24
   ```

4. **Clear Browser Cookies**
   ```bash
   # If using the frontend, clear browser cookies and try again
   ```

### Docker Conversion Issues

#### Issue: Docker conversion fails

**Symptoms:**
- Error message: `Failed to convert Docker image`
- Docker-related errors
- Timeout errors

**Solutions:**

1. **Check Docker Installation**
   ```bash
   # Verify Docker is installed and running
   docker --version
   docker ps
   ```

2. **Check Image Accessibility**
   ```bash
   # Try pulling the image manually
   docker pull alpine:latest
   ```

3. **Check Storage Space**
   ```bash
   # Ensure sufficient disk space
   df -h
   
   # Clean up Docker
   docker system prune -a
   ```

4. **Check Network Connectivity**
   ```bash
   # Ensure Docker registry is accessible
   curl -I https://registry-1.docker.io/v2/
   ```

## Frontend Issues

### Build and Deployment Issues

#### Issue: Frontend build fails

**Symptoms:**
- Error message: `Build failed`
- npm or Node.js errors
- Dependency errors

**Solutions:**

1. **Check Node.js Version**
   ```bash
   # Verify Node.js version
   node --version
   
   # Update Node.js if needed
   nvm install --lts
   ```

2. **Clean npm Cache**
   ```bash
   # Clear npm cache
   npm cache clean --force
   
   # Reinstall dependencies
   rm -rf node_modules
   npm install
   ```

3. **Check for Dependency Issues**
   ```bash
   # Check for outdated dependencies
   npm outdated
   
   # Update dependencies
   npm update
   ```

4. **Check for TypeScript Errors**
   ```bash
   # Run TypeScript compiler
   npx tsc --noEmit
   ```

#### Issue: Frontend cannot connect to server

**Symptoms:**
- Error message: `Failed to connect to server`
- API errors in browser console
- Login failures

**Solutions:**

1. **Check API URL Configuration**
   ```bash
   # Verify VITE_API_URL in .env file
   cat .env
   ```

2. **Check CORS Configuration**
   ```bash
   # Ensure server allows requests from frontend origin
   # Add appropriate headers in server response
   ```

3. **Check Network Connectivity**
   ```bash
   # Ensure server is accessible from browser
   curl -I http://localhost:3000
   ```

4. **Check Browser Console**
   ```bash
   # Open browser developer tools and check for errors
   ```

### User Interface Issues

#### Issue: UI elements not working

**Symptoms:**
- Buttons not responding
- Forms not submitting
- Visual glitches

**Solutions:**

1. **Clear Browser Cache**
   ```bash
   # Clear browser cache and reload
   ```

2. **Check Browser Compatibility**
   ```bash
   # Try a different browser
   # The frontend works best with Chrome, Firefox, or Edge
   ```

3. **Check JavaScript Console**
   ```bash
   # Open browser developer tools and check for errors
   ```

4. **Check for CSS Issues**
   ```bash
   # Inspect elements with browser developer tools
   ```

## Advanced Troubleshooting

### Debugging RFS

#### Using Debug Logs

Enable debug logging for detailed information:

```bash
# For RFS commands
rfs --debug command [options]

# For FL server
rfs server --config-path config.toml --debug
```

#### Using strace

Trace system calls to diagnose low-level issues:

```bash
# Trace RFS mount
sudo strace -f rfs mount -m flist.fl /mount/point
```

#### Using tcpdump

Capture network traffic to diagnose connectivity issues:

```bash
# Capture traffic to/from a storage backend
sudo tcpdump -i any host store.example.com -w capture.pcap
```

### Recovering from Failures

#### Corrupted Flist

If an flist is corrupted:

1. **Try to repair it**
   ```bash
   # Extract metadata if possible
   rfs config -m corrupted.fl tag list > tags.txt
   rfs config -m corrupted.fl store list > stores.txt
   ```

2. **Recreate it**
   ```bash
   # If you have the original content
   rfs pack -m new.fl -s store-url /path/to/directory
   ```

3. **Restore from backup**
   ```bash
   # If you have a backup
   cp backup.fl corrupted.fl
   ```

#### Stuck Mounts

If a mount is stuck:

1. **Force unmount**
   ```bash
   # Force unmount
   sudo umount -f /mount/point
   
   # If still stuck
   sudo umount -l /mount/point
   ```

2. **Kill RFS processes**
   ```bash
   # Find RFS processes
   ps aux | grep rfs
   
   # Kill them
   sudo kill -9 <pid>
   ```

3. **Check for open files**
   ```bash
   # Check for processes using the mount
   lsof | grep /mount/point
   ```

#### Server Recovery

If the FL server crashes:

1. **Check logs**
   ```bash
   # Check server logs
   cat server.log
   ```

2. **Check for resource issues**
   ```bash
   # Check for memory issues
   free -h
   
   # Check for disk space issues
   df -h
   ```

3. **Restart with reduced load**
   ```bash
   # Start with minimal configuration
   rfs server --config-path minimal-config.toml
   ```

## Getting Help

If you're still experiencing issues after trying the solutions in this guide:

1. **Check Documentation**
   - Review the [RFS documentation](../README.md)
   - Check the [Tutorials](../tutorials/) for step-by-step guides

2. **Check GitHub Issues**
   - Search for similar issues on the [RFS GitHub repository](https://github.com/threefoldtech/rfs/issues)
   - Open a new issue if needed

3. **Community Support**
   - Join the ThreeFold community forums
   - Ask for help in the appropriate channels

4. **Gather Information**
   - Include detailed error messages
   - Describe your environment (OS, hardware, etc.)
   - Explain the steps to reproduce the issue
   - Share relevant logs and configuration files (with sensitive information removed)

## Next Steps

For more information about related topics, see:
- [RFS CLI User Guide](./rfs-cli.md)
- [RFS Server User Guide](./fl-server.md)
- [Performance Tuning](./performance-tuning.md)