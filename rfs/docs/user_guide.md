# RFS User Guide

This document provides an overview of the commands available in the RFS application and their usage.

---

## Commands

### 1. **Mount**

Mount an FL (Flist) to a target directory.

**Usage:**

```bash
rfs mount --meta <path_to_flist> --cache <cache_directory> [--daemon] [--log <log_file>] <target>
```

**Options:**

- `--meta`: Path to the metadata file (flist).
- `--cache`: Directory used as a cache for downloaded file chunks (default: `/tmp/cache`).
- `--daemon`: Run the process in the background.
- `--log`: Log file (only used with daemon mode).
- `<target>`: Target mount point.

---

### 2. **Pack**

Create an FL and upload blocks to the provided storage.

**Usage:**

```bash
rfs pack --meta <path_to_flist> --store <store_url>... [--no-strip-password] <target_directory>
```

**Options:**

- `--meta`: Path to the metadata file (flist).
- `--store`: Store URL(s) in the format `[xx-xx=]<url>`. Multiple stores can be specified.
- `--no-strip-password`: Disable automatic password stripping from the store URL.
- `<target_directory>`: Directory to upload.

---

### 3. **Unpack**

Download the content of an FL to a specified location.

**Usage:**

```bash
rfs unpack --meta <path_to_flist> --cache <cache_directory> [--preserve-ownership] <target_directory>
```

**Options:**

- `--meta`: Path to the metadata file (flist).
- `--cache`: Directory used as a cache for downloaded file chunks (default: `/tmp/cache`).
- `--preserve-ownership`: Preserve file ownership from the FL (requires sudo).
- `<target_directory>`: Directory to unpack the content.

---

### 4. **Clone**

Copy data from the stores of an FL to another store.

**Usage:**

```bash
rfs clone --meta <path_to_flist> --store <store_url>... --cache <cache_directory>
```

**Options:**

- `--meta`: Path to the metadata file (flist).
- `--store`: Store URL(s) in the format `[xx-xx=]<url>`. Multiple stores can be specified.
- `--cache`: Directory used as a cache for downloaded file chunks (default: `/tmp/cache`).

---

### 5. **Config**

List or modify FL metadata and stores.

**Usage:**

```bash
rfs config --meta <path_to_flist> <subcommand>
```

**Subcommands:**

- `tag list`: List all tags.
- `tag add --tag <key=value>`: Add a tag.
- `tag delete --key <key>`: Delete a tag.
- `store list`: List all stores.
- `store add --store <store_url>`: Add a store.
- `store delete --store <store_url>`: Delete a store.

---

### 6. **Docker**

Convert a Docker image to an FL.

**Usage:**

```bash
rfs docker --image-name <image_name> --store <store_url>... [--username <username>] [--password <password>] [--auth <auth>] [--email <email>] [--server-address <server_address>] [--identity-token <token>] [--registry-token <token>]
```

**Options:**

- `--image-name`: Name of the Docker image to convert.
- `--store`: Store URL(s) in the format `[xx-xx=]<url>`. Multiple stores can be specified.
- Additional options for Docker credentials (e.g., `--username`, `--password`, etc.).

---

### 7. **Server**

Run the FL server.

**Usage:**

```bash
rfs server --config-path <config_file> [--debug]
```

**Options:**

- `--config-path`: Path to the server configuration file.
- `--debug`: Enable debugging logs.

---

### 8. **Upload**

Upload a file to a server.

**Usage:**

```bash
rfs upload <file_path> --server <server_url> [--block-size <size>]
```

**Options:**

- `<file_path>`: Path to the file to upload.
- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--block-size`: Block size for splitting the file (default: 1MB).

---

### 9. **UploadDir**

Upload a directory to a server.

**Usage:**

```bash
rfs upload-dir <directory_path> --server <server_url> [--block-size <size>] [--create-flist] [--flist-output <output_path>]
```

**Options:**

- `<directory_path>`: Path to the directory to upload.
- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--block-size`: Block size for splitting the files (default: 1MB).
- `--create-flist`: Create and upload an FL file.
- `--flist-output`: Path to output the FL file.

---

### 10. **Download**

Download a file from a server using its hash.

**Usage:**

```bash
rfs download  <file_hash> --output <output_file> --server <server_url>
```

**Options:**

- `<file_hash>`: Hash of the file to download.
- `--output`: Name to save the downloaded file as.
- `--server`: Server URL (e.g., `http://localhost:8080`).

---

### 11. **DownloadDir**

Download a directory from a server using its FL hash.

**Usage:**

```bash
rfs download-dir <flist_hash> --output <output_directory> --server <server_url>
```

**Options:**

- `<flist_hash>`: Hash of the FL to download.
- `--output`: Directory to save the downloaded files to.
- `--server`: Server URL (e.g., `http://localhost:8080`).

---

### 12. **Exists**

Check if a file or hash exists on the server.

**Usage:**

```bash
rfs exists <file_or_hash> --server <server_url> [--block-size <size>]
```

**Options:**

- `<file_or_hash>`: Path to the file or hash to check.
- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--block-size`: Block size for splitting the file (default: 1MB).

---

### 13. **flist create**

Creates an flist from a directory.

**Usage:**

```bash
rfs flist create <directory> --output /path/to/output.flist --server http://localhost:8080 --block-size 1048576
```

**Options:**

- `<directory>`: Path to the directory to create the flist from.
- `--output`: Path to save the generated flist file.
- `--server`: Server URL (e.g., <http://localhost:8080>).
- `--block-size`: Block size for splitting the files (default: 1MB).

---

### 14. **Website Publish**

Publish a website directory to the server.

**Usage:**

```bash
rfs website-publish <directory_path> --server <server_url> [--block-size <size>]
```

**Options:**

- `<directory_path>`: Path to the website directory to publish.
- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--block-size`: Block size for splitting the files (default: 1MB).

---

### 15. **Token**

Retrieve an authentication token using username and password.

**Usage:**

```bash
rfs token --username <username> --password <password> --server <server_url>
```

**Options:**

- `--username`: Username for authentication.
- `--password`: Password for authentication.
- `--server`: Server URL (e.g., `http://localhost:8080`).

---

### 16. **Track**

Track user blocks on the server and their download statistics.

**Usage:**

```bash
rfs track --server <server_url> --token <auth_token> [--details]
```

**Options:**

- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--token`: Authentication token for the server.
- `--details`: Display detailed information about each block.

---

### 17. **TrackBlocks**

Track download statistics for specific blocks or all blocks.

**Usage:**

```bash
rfs track-blocks --server <server_url> --token <auth_token> [--hash <block_hash>] [--all] [--details]
```

**Options:**

- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--token`: Authentication token for the server.
- `--hash`: Specific block hash to track (conflicts with --all).
- `--all`: Track all blocks (default if no hash is provided).
- `--details`: Display detailed information about each block.

---

### 18. **TrackWebsite**

Track download statistics for a website using its flist hash.

**Usage:**

```bash
rfs track-website <flist_hash> --server <server_url> [--details]
```

**Options:**

- `<flist_hash>`: Hash of the website's flist.
- `--server`: Server URL (e.g., `http://localhost:8080`).
- `--details`: Display detailed information about each block.

---

### Examples

1. **Upload a File**:

   Upload a file to the server with a custom block size:

   ```bash
   rfs upload big_file.txt --server http://localhost:8080 --block-size 2097152
   ```

2. **Download a Directory**:

   Download a directory from the server using its FL hash:

   ```bash
   rfs download-dir abc123 --output ./mydir --server http://localhost:8080
   ```

3. **Pack a Directory**:

   Create an FL from a directory and upload it to a specific store:

   ```bash
   rfs pack --meta myflist.fl --store http://store.url --target ./mydir
   ```

4. **Unpack an FL**:

   Unpack the contents of an FL to a target directory while preserving file ownership:

   ```bash
   rfs unpack --meta myflist.fl --cache /tmp/cache --preserve-ownership --target ./output
   ```

5. **Convert a Docker Image to an FL**:

   Convert a Docker image to an FL and upload it to a store with authentication:

   ```bash
   rfs docker --image-name redis --store server://http://localhost:4000 --username myuser --password mypass
   ```

6. **Publish a Website**:

   Publish a website directory to the server:

   ```bash
   rfs website-publish ./website --server http://localhost:8080
   ```

7. **Check if a File Exists**:

   Verify if a file exists on the server using its hash:

   ```bash
   rfs exists myfilehash --server http://localhost:8080
   ```

8. **Create an FL from a Directory**:

   Create an FL from a directory and save it to a specific output path:

   ```bash
   rfs flist create ./mydir --output ./mydir.flist --server http://localhost:8080
   ```

9. **Run the FL Server**:

   Start the FL server with a specific configuration file:

   ```bash
   rfs server --config-path ./config.yaml --debug
   ```

10. **List FL Metadata Tags**:

    List all tags in an FL metadata file:

    ```bash
    rfs config --meta myflist.fl tag list
    ```

11. **Add a Tag to FL Metadata**:

    Add a custom tag to an FL metadata file:

    ```bash
    rfs config --meta myflist.fl tag add --tag key=value
    ```

12. **Delete a Tag from FL Metadata**:

    Remove a specific tag from an FL metadata file:

    ```bash
    rfs config --meta myflist.fl tag delete --key key
    ```

13. **Clone an FL to Another Store**:

    Clone the data of an FL to another store:

    ```bash
    rfs clone --meta myflist.fl --store http://newstore.url --cache /tmp/cache
    ```

14.  **Get an Authentication Token**:

    Retrieve an authentication token from the server:

    ```bash
    rfs token --username myuser --password mypass --server http://localhost:8080
    ```

15.  **Track User Blocks**:

    Track all blocks uploaded by the authenticated user:

    ```bash
    rfs track --server http://localhost:8080 --token mytoken
    ```

16.  **Track a Specific Block**:

    Track download statistics for a specific block:

    ```bash
    rfs track-blocks --server http://localhost:8080 --hash abc123def456
    ```

4.  **Track Website Downloads**:

    Track download statistics for a published website:

    ```bash
    rfs track-website abc123def456 --server http://localhost:8080 --details
    ```

---

For more details, refer to the documentation or use the `--help` flag with any command.
