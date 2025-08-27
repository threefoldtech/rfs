# RFS Web Interface User Guide

This guide provides detailed information about using the web interface that is served by the RFS server.

## Introduction

When running the `rfs server` command, it serves a web interface that provides a graphical user interface for managing flists. This web interface allows users to:

- Log in to the server
- Create flists from Docker images
- View and manage their flists
- Download flists

The web interface communicates with the server through its REST API, providing a user-friendly interface for operations that would otherwise require command-line usage or direct API calls.

## Accessing the Web Interface

### Prerequisites

Before accessing the web interface, ensure you have:

- A running RFS server (see the [Server Setup Tutorial](../tutorials/server-setup.md))
- A web browser (Chrome, Firefox, Safari, or Edge)
- User credentials configured in the server's configuration file

### Accessing the Interface

To access the web interface, open a web browser and navigate to the server's URL:

```
http://hostname:port
```

Replace `hostname` and `port` with the actual hostname and port of your RFS server. For example:

```
http://localhost:3000
```

## Using the Web Interface

### Logging In

1. Navigate to the server URL in your web browser
2. Enter your username and password
3. Click "Login"

The login credentials are the same as those configured in the server's configuration file.

### Creating a Flist from a Docker Image

1. Log in to the web interface
2. Navigate to the "Create Flist" page
3. Enter the Docker image name (e.g., `alpine:latest`)
4. Click "Create"
5. Wait for the creation process to complete

The web interface will display the progress of the flist creation and notify you when it's complete.

### Viewing Your Flists

1. Log in to the web interface
2. Navigate to the "My Flists" page

This page displays a list of all your flists, including:
- Flist name
- Creation date
- Size
- Actions (preview, download)

### Previewing a Flist

1. Log in to the web interface
2. Navigate to the "My Flists" page
3. Click the "Preview" button for the flist you want to preview

The preview shows the directory structure and metadata of the flist.

### Downloading a Flist

1. Log in to the web interface
2. Navigate to the "My Flists" page
3. Click the "Download" button for the flist you want to download

The flist will be downloaded to your computer.

## User Interface Overview

### Home Page

The home page provides an overview of the RFS system and links to the main features.

### Login Page

The login page allows you to authenticate with the server.

### Create Flist Page

The Create Flist page provides a form for creating flists from Docker images.

### My Flists Page

The My Flists page displays a list of your flists and allows you to manage them.

### Preview Flist Page

The Preview Flist page displays the contents and metadata of a specific flist.

## Troubleshooting

### Connection Issues

If you can't connect to the server:

1. **Check the Server**: Ensure the RFS server is running
   ```bash
   # Check if the server process is running
   ps aux | grep "rfs server"
   ```

2. **Check the URL**: Ensure you're using the correct hostname and port
   ```bash
   # Test the connection
   curl -I http://hostname:port
   ```

3. **Check Firewall Rules**: Ensure the server port is accessible
   ```bash
   # Check if the port is open
   telnet hostname port
   ```

### Authentication Issues

If you can't log in:

1. **Check Credentials**: Ensure your username and password are correct
2. **Check the Server Configuration**: Ensure your user is configured in the server's configuration file
3. **Check the Network Tab**: Look for authentication errors in the browser's developer tools

### Flist Creation Issues

If flist creation fails:

1. **Check the Docker Image**: Ensure the Docker image exists and is accessible
2. **Check the Server Logs**: Look for errors in the server logs
3. **Check Storage Space**: Ensure there's sufficient storage space on the server

## Advanced Usage

### API Integration

The web interface communicates with the server using its REST API. If you're developing custom integrations, you can use the same API.

See the [RFS Server User Guide](./fl-server.md) for API documentation.

### Using with Custom Server Configuration

If your server is configured with custom settings (e.g., different port, HTTPS, etc.), you'll need to adjust the URL accordingly:

```
https://hostname:custom-port
```

### Using Behind a Reverse Proxy

If your server is behind a reverse proxy (e.g., Nginx, Caddy), you'll access the web interface through the proxy URL:

```
https://your-domain.example.com
```

## Next Steps

For more information about related topics, see:
- [RFS Server User Guide](./fl-server.md)
- [RFS CLI User Guide](./rfs-cli.md)
- [Tutorials](../tutorials/)