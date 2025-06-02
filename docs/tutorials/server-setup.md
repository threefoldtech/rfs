# Setting Up the RFS Server

This tutorial will guide you through the process of setting up and configuring the server functionality of the RFS command, which provides a REST API and web interface for managing flists.

## Prerequisites

Before you begin, make sure you have:

- RFS installed (see the [Getting Started](./getting-started.md) tutorial)
- Basic understanding of server configuration
- A machine with sufficient resources (CPU, RAM, storage)
- Network connectivity for the server

## Introduction to RFS Server

The server functionality of the RFS command provides:

- A REST API for managing flists
- User authentication and authorization
- Docker image to flist conversion
- Flist storage and retrieval
- Integration with the web frontend

## Basic Server Setup

### 1. Create a Configuration File

First, create a configuration file for the server. Create a file named `config.toml` with the following content:

```toml
host = "localhost"
port = 3000
store_url = ["dir:///tmp/store0"]
flist_dir = "flists"

jwt_secret = "your-secret-key"
jwt_expire_hours = 5

[[users]]
username = "admin"
password = "admin-password"

[[users]]
username = "user1"
password = "user1-password"
```

Customize the configuration:
- `host`: The hostname or IP address to bind to
- `port`: The port to listen on
- `store_url`: A list of storage backends to use
- `flist_dir`: The directory to store flists in
- `jwt_secret`: A secret key for JWT token generation
- `jwt_expire_hours`: The lifetime of JWT tokens in hours
- `users`: A list of authorized users with usernames and passwords

### 2. Create the Flist Directory Structure

Create the directory structure for storing flists:

```bash
# Create the main flists directory
mkdir -p flists

# Create subdirectories for each user
mkdir -p flists/admin flists/user1
```

### 3. Create the Store Directory

If you're using a directory store, create the directory:

```bash
mkdir -p /tmp/store0
```

### 4. Run the RFS Server

Run the server using the configuration file:

```bash
rfs server --config-path config.toml
```

For debugging, you can add the `--debug` flag:

```bash
rfs server --config-path config.toml --debug
```

The server should start and listen on the configured port.

## Advanced Server Configuration

### Using Remote Storage Backends

For production use, you'll typically want to use a remote storage backend:

```toml
# Using a ZDB backend
store_url = ["zdb://zdb.example.com:9900/namespace"]

# Using an S3 backend
store_url = ["s3://username:password@s3.example.com:9000/bucket"]
```

### Multiple Storage Backends

You can configure multiple storage backends for sharding or replication:

```toml
# Sharding across two stores
store_url = [
    "00-80=dir:///tmp/store1",
    "81-ff=dir:///tmp/store2"
]

# Using multiple backend types
store_url = [
    "dir:///tmp/store0",
    "zdb://zdb.example.com:9900/namespace",
    "s3://username:password@s3.example.com:9000/bucket"
]
```

### HTTPS Configuration

For production use, you should configure HTTPS. The RFS server doesn't directly support HTTPS, but you can use a reverse proxy like Nginx or Caddy:

#### Using Caddy

Create a `Caddyfile`:

```
rfs.example.com {
    reverse_proxy localhost:3000
}
```

Run Caddy:

```bash
caddy run
```

#### Using Nginx

Create an Nginx configuration:

```nginx
server {
    listen 443 ssl;
    server_name rfs.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Reload Nginx:

```bash
nginx -s reload
```

## Running as a Service

For production use, you'll want to run the RFS server as a service.

### Using Systemd

Create a systemd service file at `/etc/systemd/system/rfs-server.service`:

```ini
[Unit]
Description=RFS Server
After=network.target

[Service]
ExecStart=/usr/local/bin/rfs server --config-path /etc/rfs-server/config.toml
WorkingDirectory=/var/lib/rfs-server
User=rfs-server
Group=rfs-server
Restart=always

[Install]
WantedBy=multi-user.target
```

Create the necessary directories and user:

```bash
# Create the user
sudo useradd -r -s /bin/false rfs-server

# Create the directories
sudo mkdir -p /etc/rfs-server /var/lib/rfs-server/flists

# Copy the configuration
sudo cp config.toml /etc/rfs-server/

# Set permissions
sudo chown -R rfs-server:rfs-server /etc/rfs-server /var/lib/rfs-server
```

Enable and start the service:

```bash
sudo systemctl enable rfs-server
sudo systemctl start rfs-server
```

## Docker Deployment

You can also deploy the RFS server using Docker.

### Using the Provided Dockerfile

The RFS repository includes a Dockerfile for the server:

```bash
# Build the Docker image
docker build -t rfs-server -f Dockerfile .

# Run the container
docker run -d \
  -p 3000:3000 \
  -v $(pwd)/config.toml:/app/config.toml \
  -v $(pwd)/flists:/app/flists \
  -v /tmp/store0:/tmp/store0 \
  --name rfs-server \
  rfs-server
```

### Using Docker Compose

Create a `docker-compose.yml` file:

```yaml
version: '3'
services:
  rfs-server:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    volumes:
      - ./config.toml:/app/config.toml
      - ./flists:/app/flists
      - /tmp/store0:/tmp/store0
    restart: always
```

Run with Docker Compose:

```bash
docker-compose up -d
```

## Setting Up the Frontend

The RFS server serves a web interface that provides a graphical user interface for managing flists.

### Accessing the Frontend

Once the server is running, you can access the frontend by opening a web browser and navigating to the server URL:

```
http://localhost:3000
```

### Using the Frontend

1. Log in using one of the configured user accounts.
2. Use the interface to:
   - Create flists from Docker images
   - View and manage your flists
   - Download flists

## Testing the Setup

### 1. Access the Frontend

Open a web browser and navigate to:

```
http://localhost:3000
```

### 2. Log In

Log in using one of the configured user accounts.

### 3. Create a Flist

Try creating a flist from a Docker image:

1. Click on "Create Flist"
2. Enter a Docker image name (e.g., `alpine:latest`)
3. Click "Create"

### 4. View and Download Flists

1. Click on "My Flists"
2. View the list of your flists
3. Click on a flist to preview or download it

## Troubleshooting

### Server Won't Start

If the server won't start, check:

1. **Configuration File**: Ensure the configuration file is valid TOML
   ```bash
   # Validate the TOML file
   cat config.toml | python3 -c "import tomli; import sys; tomli.loads(sys.stdin.read())"
   ```

2. **Permissions**: Ensure the server has permission to access the directories
   ```bash
   # Check permissions
   ls -la flists /tmp/store0
   ```

3. **Port Availability**: Ensure the configured port is available
   ```bash
   # Check if the port is in use
   netstat -tuln | grep 3000
   ```

### Authentication Issues

If you can't log in:

1. **User Configuration**: Ensure the user is correctly configured in `config.toml`
2. **JWT Secret**: Ensure the JWT secret is set and consistent
3. **Server Logs**: Check the server logs for authentication errors

### Docker Conversion Issues

If Docker conversion fails:

1. **Docker Availability**: Ensure Docker is installed and running
2. **Image Accessibility**: Ensure the Docker image is accessible
3. **Storage Space**: Ensure there's sufficient storage space

## Next Steps

Now that you have the RFS server set up, you might want to learn:

- [Using the Web Interface](../user-guides/frontend.md)
- [RFS Server API Reference](../user-guides/fl-server.md)
- [Performance Tuning](../user-guides/performance-tuning.md)