# mycofs

`mycofs` is a binary that provides the same functionality as the `rfs` binary. It's built on top of the `rfs` library and provides the same commands and options.

## Overview

`mycofs` is a tool for working with flists (file lists), which are metadata files that describe a filesystem. It allows you to:

- Mount flists as a filesystem
- Create flists from a directory
- Unpack flists to a directory
- Clone flists from one storage to another
- Configure flist metadata and stores

## Commands

### Mount

Mount an FL (flist) as a filesystem:

```bash
mycofs mount -m <meta-file> -c <cache-dir> [--daemon] [-l <log-file>] <target>
```

**Example:**

```bash
# Mount an flist to /mnt/myfs with cache in /tmp/mycache
mycofs mount -m alpine-latest.fl -c /tmp/mycache /mnt/myfs

# Mount as a daemon with logging
mycofs mount -m alpine-latest.fl -c /tmp/mycache -d -l /var/log/mycofs.log /mnt/myfs
```

### Pack

Create an FL and upload blocks to provided storage:

```bash
mycofs pack -m <meta-file> -s <store-url> [--no-strip-password] <target>
```

**Example:**

```bash
# Pack a directory to an flist and store blocks in a local directory
mycofs pack -m myapp.fl -s "dir:///tmp/blocks" /path/to/myapp

# Pack with multiple storage backends
mycofs pack -m myapp.fl -s "00-7f=dir:///tmp/blocks1" -s "80-ff=dir:///tmp/blocks2" /path/to/myapp

# Pack with S3 storage
mycofs pack -m myapp.fl -s "s3://accesskey:secretkey@endpoint/bucket" /path/to/myapp
```

### Unpack

Unpack (download) content of an FL to the provided location:

```bash
mycofs unpack -m <meta-file> -c <cache-dir> [-p] <target>
```

**Example:**

```bash
# Unpack an flist to /tmp/myapp
mycofs unpack -m myapp.fl -c /tmp/cache /tmp/myapp

# Unpack with preserved ownership (requires sudo)
sudo mycofs unpack -m myapp.fl -c /tmp/cache -p /tmp/myapp
```

### Clone

Clone copies the data from the stores of an FL to another stores:

```bash
mycofs clone -m <meta-file> -s <store-url> -c <cache-dir>
```

**Example:**

```bash
# Clone from the flist's current stores to a local directory
mycofs clone -m myapp.fl -s "dir:///tmp/newblocks" -c /tmp/cache

# Clone to multiple storage backends
mycofs clone -m myapp.fl -s "00-7f=dir:///tmp/newblocks1" -s "80-ff=dir:///tmp/newblocks2" -c /tmp/cache

# Clone to S3 storage
mycofs clone -m myapp.fl -s "s3://accesskey:secretkey@endpoint/bucket" -c /tmp/cache
```

### Docker

Convert a Docker image to an FL:

```bash
mycofs docker -i <image-name> -s <store-url> [--username <username>] [--password <password>]
```

**Example:**

```bash
# Convert an Alpine Docker image to an FL
mycofs docker -i alpine:latest -s "dir:///tmp/blocks"

# Convert with Docker Hub credentials
mycofs docker -i myregistry/myapp:latest -s "dir:///tmp/blocks" --username myuser --password mypass

# Convert with multiple storage backends
mycofs docker -i ubuntu:20.04 -s "00-7f=dir:///tmp/blocks1" -s "80-ff=dir:///tmp/blocks2"
```

### Server

Run the fl-server with a specified configuration file:

```bash
mycofs server -c <config-path> [-d] [-d -d]
```

**Example:**

```bash
# Run the fl-server with a specific config file
mycofs server -c /etc/fl-server/config.toml

# Run with a local config file
mycofs server -c ./config.toml

# Run with debug logging enabled
mycofs server -c ./config.toml -d

# Run with trace logging enabled (more verbose)
mycofs server -c ./config.toml -d -d
```

### Config

List or modify FL metadata and stores:

```bash
mycofs config -m <meta-file> <subcommand>
```

**Examples:**

**Tag operations:**

```bash
# List all tags
mycofs config -m myapp.fl tag list

# Add tags
mycofs config -m myapp.fl tag add -t "version=1.0.0" -t "author=John Doe"

# Delete a specific tag
mycofs config -m myapp.fl tag delete -k "version"

# Delete all tags
mycofs config -m myapp.fl tag delete --all
```

**Store operations:**

```bash
# List all stores
mycofs config -m myapp.fl store list

# Add stores
mycofs config -m myapp.fl store add -s "dir:///tmp/blocks"
mycofs config -m myapp.fl store add -s "00-7f=dir:///tmp/blocks1" -s "80-ff=dir:///tmp/blocks2"

# Delete a specific store
mycofs config -m myapp.fl store delete -s "dir:///tmp/blocks"

# Delete all stores
mycofs config -m myapp.fl store delete --all
```

## Building

To build `mycofs`, run:

```bash
# Debug build
cargo build --bin mycofs

# Release build with musl target (static binary)
cargo build --bin mycofs --release --target x86_64-unknown-linux-musl
```

The binary will be available at:

- Debug build: `target/debug/mycofs`
- Release build: `target/x86_64-unknown-linux-musl/release/mycofs`

## Usage

For detailed usage information, run:

```bash
mycofs --help
```

```bash
Commands:
  mount   mount an FL
  pack    create an FL and upload blocks to provided storage
  unpack  unpack (downloads) content of an FL the provided location
  clone   clone copies the data from the stores of an FL to another stores
  config  list or modify FL metadata and stores
  docker  convert a docker image to an FL
  server  run the fl-server
  help    Print this message or the help of the given subcommand(s)

Options:
      --debug...  enable debugging logs
  -h, --help      Print help
  -V, --version   Print version
```

Or for a specific command:

```bash
mycofs <command> --help
