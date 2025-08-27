# Getting Started with RFS

This tutorial will guide you through the process of installing and setting up RFS, and performing basic operations.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust and Cargo**: RFS is written in Rust, so you'll need the Rust toolchain.
- **Build Essentials**: Required for compiling RFS and its dependencies.
- **FUSE**: Required for mounting flists.

## Installation

### 1. Install Dependencies

#### On Ubuntu/Debian:

```bash
# Install build essentials
sudo apt-get update
sudo apt-get install -y build-essential

# Install FUSE
sudo apt-get install -y fuse libfuse-dev

# Install musl-tools (for static compilation)
sudo apt-get install -y musl-tools
```

#### On Fedora/CentOS/RHEL:

```bash
# Install build essentials
sudo dnf install -y gcc make

# Install FUSE
sudo dnf install -y fuse fuse-devel

# Install musl-tools (for static compilation)
sudo dnf install -y musl-devel
```

### 2. Install Rust

If you don't have Rust installed, you can install it using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Clone the RFS Repository

```bash
git clone https://github.com/threefoldtech/rfs.git
cd rfs
```

### 4. Build RFS

```bash
# Add the musl target
rustup target add x86_64-unknown-linux-musl

# Build the binaries
cargo build --features build-binary --release --target=x86_64-unknown-linux-musl
```

### 5. Install the Binaries

```bash
# Copy the binaries to a location in your PATH
sudo cp ./target/x86_64-unknown-linux-musl/release/rfs /usr/local/bin/
```

## Basic Usage

Now that you have RFS installed, let's go through some basic operations.

### Creating a Local Store

First, let's create a directory to use as a local store:

```bash
mkdir -p ~/rfs-store
```

### Creating an Flist

Let's create an flist from a directory:

```bash
# Create a test directory with some files
mkdir -p ~/test-dir
echo "Hello, world!" > ~/test-dir/hello.txt
echo "Another file" > ~/test-dir/another.txt
mkdir -p ~/test-dir/subdir
echo "Subdirectory file" > ~/test-dir/subdir/file.txt

# Create an flist from the test directory
rfs pack -m ~/test.fl -s dir://~/rfs-store ~/test-dir
```

This command creates an flist named `test.fl` in your home directory, using the directory store at `~/rfs-store`.

### Examining the Flist

You can examine the contents of the flist using the `config` command:

```bash
# List the tags in the flist
rfs config -m ~/test.fl tag list

# List the stores in the flist
rfs config -m ~/test.fl store list
```

### Mounting the Flist

Now, let's mount the flist to access its contents:

```bash
# Create a mount point
mkdir -p ~/mount-point

# Mount the flist
sudo rfs mount -m ~/test.fl -c ~/rfs-cache ~/mount-point
```

This command mounts the flist at `~/mount-point`, using `~/rfs-cache` as a cache directory for downloaded content.

### Accessing the Mounted Flist

You can now access the files in the mounted flist:

```bash
# List the contents of the mount point
ls -la ~/mount-point

# Read a file from the mount point
cat ~/mount-point/hello.txt
```

### Unmounting the Flist

When you're done, you can unmount the flist:

```bash
sudo umount ~/mount-point
```

### Extracting the Flist

Instead of mounting, you can also extract the contents of the flist to a directory:

```bash
# Create a destination directory
mkdir -p ~/extracted

# Extract the flist
rfs unpack -m ~/test.fl -c ~/rfs-cache ~/extracted
```

This command extracts the contents of the flist to `~/extracted`.

## Next Steps

Now that you've learned the basics of RFS, you can explore more advanced features:

- [Creating Flists from Docker Images](./docker-conversion.md)
- [Using Remote Storage Backends](../user-guides/storage-backends.md)
- [Setting Up the FL Server](./server-setup.md)

For a complete reference of RFS commands, see the [RFS CLI User Guide](../user-guides/rfs-cli.md).