# RFS Server User Guide

This guide provides detailed information about configuring, running, and using the server functionality of the RFS command.

## Introduction

The server functionality of the RFS command provides a REST API for managing flists. It allows users to:

- Create flists from Docker images
- Store and retrieve flists
- Manage flist metadata
- Authenticate users

The server also serves a web interface for these operations, making it easier to manage flists through a graphical user interface.

## Configuration

### Configuration File

The server is configured using a TOML file. Here's an example configuration:

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

### Configuration Options

| Option | Description | Default | Required |
|--------|-------------|---------|----------|
| `host` | Hostname or IP address to bind to | - | Yes |
| `port` | Port to listen on | - | Yes |
| `store_url` | List of storage backends to use | - | Yes |
| `flist_dir` | Directory to store flists in | - | Yes |
| `jwt_secret` | Secret key for JWT token generation | - | Yes |
| `jwt_expire_hours` | Lifetime of JWT tokens in hours | - | Yes |
| `users` | List of authorized users | - | Yes |

### User Configuration

Users are configured in the `users` section of the configuration file:

```toml
[[users]]
username = "username"
password = "password"
```

Each user must have a unique username and a password.

## Running the Server

To run the server, use the `server` subcommand of the RFS tool:

```bash
rfs server --config-path config.toml
```

For debugging, add the `--debug` flag:

```bash
rfs server --config-path config.toml --debug
```

### Directory Structure

Before running the server, ensure the following directory structure exists:

```
flists/
├── user1/
├── user2/
└── ...
```

Create a directory for each configured user within the `flists` directory (or the directory specified in `flist_dir`).

## API Reference

The server provides a REST API for managing flists. All API endpoints require authentication except for the login endpoint.

### Authentication

#### Login

```
POST /auth/login
```

**Request Body:**
```json
{
  "username": "user1",
  "password": "password1"
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

Use this token in the `Authorization` header for subsequent requests:

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Flist Management

#### List User Flists

```
GET /flists
```

**Response:**
```json
{
  "flists": [
    {
      "name": "alpine-latest.fl",
      "size": 12345,
      "created": "2023-01-01T12:00:00Z"
    },
    ...
  ]
}
```

#### Create Flist from Docker Image

```
POST /flists/docker
```

**Request Body:**
```json
{
  "image": "alpine:latest"
}
```

**Response:**
```json
{
  "status": "success",
  "message": "Flist created successfully",
  "flist": "alpine-latest.fl"
}
```

#### Download Flist

```
GET /flists/{flist_name}/download
```

**Response:** The flist file as a download.

#### Get Flist Info

```
GET /flists/{flist_name}/info
```

**Response:**
```json
{
  "name": "alpine-latest.fl",
  "size": 12345,
  "created": "2023-01-01T12:00:00Z",
  "tags": {
    "docker:env": "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    ...
  }
}
```

### Block Management

#### Upload Block

```
POST /blocks
```

**Request Body:** The block content as binary data.

**Response:**
```json
{
  "hash": "abc123..."
}
```

#### Download Block

```
GET /blocks/{hash}
```

**Response:** The block content as binary data.

#### Check if Block Exists

```
HEAD /blocks/{hash}
```

**Response:** 200 OK if the block exists, 404 Not Found otherwise.

### Website Management

#### Publish Website

```
POST /websites
```

**Request Body:** The website files as a multipart form.

**Response:**
```json
{
  "status": "success",
  "message": "Website published successfully",
  "url": "http://example.com/websites/abc123"
}
```

## Using the API

### Authentication Flow

1. Obtain a JWT token by logging in:
   ```bash
   curl -X POST http://localhost:3000/auth/login \
     -H "Content-Type: application/json" \
     -d '{"username":"user1","password":"password1"}'
   ```

2. Use the token in subsequent requests:
   ```bash
   curl -X GET http://localhost:3000/flists \
     -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
   ```

### Creating a Flist from Docker

```bash
curl -X POST http://localhost:3000/flists/docker \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{"image":"nginx:latest"}'
```

### Downloading a Flist

```bash
curl -X GET http://localhost:3000/flists/nginx-latest.fl/download \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -o nginx-latest.fl
```

## Integration with Web Interface

The server serves a web interface that provides a graphical user interface for managing flists. The web interface communicates with the server through the REST API.

To access the web interface, open a browser and navigate to the server URL (e.g., `http://localhost:3000`).

For more information about using the web interface, see the [Web Interface User Guide](./frontend.md).

## Security Considerations

### JWT Secret

The `jwt_secret` in the configuration file is used to sign JWT tokens. It should be:

- Long and random
- Kept secret
- Changed periodically

### User Authentication

User credentials are stored in the configuration file. Consider:

- Using strong passwords
- Limiting access to the configuration file
- Implementing more advanced authentication methods for production use

### Storage Backend Security

Ensure that the storage backends configured in `store_url` are properly secured:

- Use authentication for write access
- Consider using read-only access for public content
- Regularly audit access to storage backends

## Troubleshooting

### Server Won't Start

If the server won't start, check:

1. **Configuration File**: Ensure the configuration file is valid TOML
   ```bash
   cat config.toml | python3 -c "import tomli; import sys; tomli.loads(sys.stdin.read())"
   ```

2. **Permissions**: Ensure the server has permission to access the directories
   ```bash
   ls -la flists
   ```

3. **Port Availability**: Ensure the configured port is available
   ```bash
   netstat -tuln | grep 3000
   ```

### Authentication Issues

If you can't authenticate:

1. **User Configuration**: Ensure the user is correctly configured in `config.toml`
2. **JWT Secret**: Ensure the JWT secret is set and consistent
3. **Token Expiration**: Check if the token has expired (default is 5 hours)

### Docker Conversion Issues

If Docker conversion fails:

1. **Docker Availability**: Ensure Docker is installed and running
   ```bash
   docker --version
   docker ps
   ```

2. **Image Accessibility**: Ensure the Docker image is accessible
   ```bash
   docker pull alpine:latest
   ```

3. **Storage Space**: Ensure there's sufficient storage space
   ```bash
   df -h
   ```

## Next Steps

For more information about related topics, see:
- [RFS CLI User Guide](./rfs-cli.md)
- [Web Interface User Guide](./frontend.md)
- [Performance Tuning](./performance-tuning.md)