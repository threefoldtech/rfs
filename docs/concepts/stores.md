# Understanding Storage Backends (Stores)

This document explains the concept of storage backends (stores) in the RFS ecosystem, how they work, and the different types available.

## What is a Storage Backend?

In the RFS ecosystem, a storage backend (or "store") is a system that stores the actual content of files referenced in flists. While flists contain metadata about files and directories, the actual file content is stored separately in one or more storage backends.

This separation of metadata and content is a fundamental design principle of RFS, enabling efficient distribution and access to filesystem data.

## Key Concepts

### Content Addressing

RFS uses content addressing to identify and store file content:

1. **Hashing**: File content is hashed using a cryptographic hash function (currently SHA-256).
2. **Chunking**: Large files are split into chunks, and each chunk is hashed separately.
3. **Deduplication**: Identical content (with the same hash) is stored only once, even if it appears in multiple files.

### Store URLs

Storage backends are identified by URLs with specific schemas:

```
schema://[credentials@]hostname[:port][/path][?parameters]
```

For example:
- `dir:///tmp/store`
- `zdb://zdb.example.com:9900/namespace`
- `s3://username:password@s3.example.com:9000/bucket`
- `http://store.example.com/content`

### Sharding

Sharding distributes content across multiple storage backends based on content hashes:

```
xx-yy=store_url
```

Where:
- `xx` is the start of the hash range (in hex)
- `yy` is the end of the hash range (in hex)
- `store_url` is the URL of the storage backend

For example:
```
00-7f=dir:///tmp/store1 80-ff=dir:///tmp/store2
```

### Replication

Replication stores the same content in multiple storage backends for redundancy:

```
00-ff=store_url1 00-ff=store_url2
```

This ensures that content is available even if one storage backend fails.

## Types of Storage Backends

RFS supports several types of storage backends, each with its own characteristics and use cases.

### 1. Directory Store (`dir`)

The directory store is the simplest backend, storing content in a local directory.

#### URL Format
```
dir:///path/to/store
```

#### Characteristics
- **Local Only**: Content is only accessible on the local machine
- **Simple**: No additional setup required
- **Fast**: Direct filesystem access
- **Ideal For**: Testing, development, and local use cases

#### Implementation
The directory store maps content hashes to files within the specified directory. Each chunk of file content is stored as a separate file, with the filename derived from the content hash.

### 2. Zero-DB Store (`zdb`)

The Zero-DB (ZDB) store uses [0-DB](https://github.com/threefoldtech/0-db), an append-only key-value store with a Redis-like API.

#### URL Format
```
zdb://hostname[:port][/namespace][?password=password]
```

#### Characteristics
- **Networked**: Content can be accessed over the network
- **Append-Only**: Data is never modified or deleted
- **Efficient**: Optimized for storing and retrieving immutable data
- **Namespaces**: Supports organizing data into namespaces
- **Ideal For**: Distributed deployments and production use

#### Implementation
The ZDB store maps content hashes to keys in the ZDB database. Each chunk of file content is stored as a value associated with a key derived from the content hash.

#### Security
ZDB supports namespaces with different access modes:
- **Public**: Anyone can read, but only authorized users can write
- **Private**: Only authorized users can read and write

### 3. S3 Store (`s3`)

The S3 store uses Amazon S3 or compatible services (like MinIO) to store content.

#### URL Format
```
s3://username:password@host:port/bucket-name[?region=region-name]
```

#### Characteristics
- **Widely Available**: Works with AWS S3 and compatible services
- **Scalable**: Can handle large amounts of data
- **Durable**: Built-in redundancy and durability
- **Configurable**: Supports various access policies
- **Ideal For**: Cloud deployments and large-scale storage

#### Implementation
The S3 store maps content hashes to objects in an S3 bucket. Each chunk of file content is stored as a separate object, with the object key derived from the content hash.

#### Security
Access to S3 buckets can be controlled through:
- **Credentials**: Username and password for authenticated access
- **Bucket Policies**: For fine-grained access control
- **Public Access**: Buckets can be configured for public read access

### 4. HTTP Store (`http`)

The HTTP store is used for retrieving content through HTTP requests. It does not support uploading, only fetching data.

#### URL Format
```
http://hostname[:port]/path
https://hostname[:port]/path
```

#### Characteristics
- **Read-Only**: Only supports retrieving content, not storing it
- **Widely Accessible**: Content can be accessed from anywhere with HTTP
- **Cacheable**: HTTP caching mechanisms can be leveraged
- **Ideal For**: Distribution of content to end-users

#### Implementation
The HTTP store maps content hashes to URLs. Each chunk of file content is retrieved by making an HTTP GET request to a URL derived from the content hash.

## Store Router

The Store Router is a component that manages the routing of content to and from the appropriate storage backends based on the content hash.

### Functionality

- **Store Selection**: Determines which storage backend(s) to use for a given content hash
- **Read Strategy**: Implements strategies for reading from multiple backends (e.g., try first available)
- **Write Strategy**: Implements strategies for writing to multiple backends (e.g., write to all)
- **Error Handling**: Manages errors and retries when interacting with storage backends

## Password Handling

When creating flists, passwords in storage URLs are typically stripped to prevent unauthorized write access. This behavior can be disabled with the `--no-strip-password` flag.

For example, if you create an flist with:
```
rfs pack -m output.fl -s s3://username:password@s3.example.com:9000/bucket /path/to/directory
```

The password will be stripped from the flist, and users of the flist will only have read access to the content.

## Use Cases and Best Practices

### Local Development

For local development and testing, the directory store is the simplest option:

```bash
# Create a local store
mkdir -p /tmp/store

# Create an flist using the local store
rfs pack -m output.fl -s dir:///tmp/store /path/to/directory
```

### Production Deployments

For production deployments, ZDB or S3 stores are recommended:

```bash
# Using ZDB
rfs pack -m output.fl -s zdb://zdb.example.com:9900/namespace /path/to/directory

# Using S3
rfs pack -m output.fl -s s3://username:password@s3.example.com:9000/bucket /path/to/directory
```

### High Availability

For high availability, use replication across multiple storage backends:

```bash
# Replicate across two ZDB instances
rfs pack -m output.fl \
  -s zdb://zdb1.example.com:9900/namespace \
  -s zdb://zdb2.example.com:9900/namespace \
  /path/to/directory
```

### Performance Optimization

For performance optimization, use sharding across multiple storage backends:

```bash
# Shard across two ZDB instances
rfs pack -m output.fl \
  -s 00-7f=zdb://zdb1.example.com:9900/namespace \
  -s 80-ff=zdb://zdb2.example.com:9900/namespace \
  /path/to/directory
```

### Content Distribution

For wide distribution of content, use HTTP stores:

```bash
# Add an HTTP store to an existing flist
rfs config -m output.fl store add -s http://store.example.com/content
```

## Security Considerations

### Access Control

Ensure that storage backends have appropriate access controls:

- **Read Access**: Configure storage backends to allow read access to users of the flist
- **Write Access**: Restrict write access to authorized users only
- **Public Content**: For public content, ensure that read access is unrestricted

### Credential Management

Be careful with storage credentials:

- **Password Stripping**: Use the default password stripping behavior unless you have a specific reason not to
- **Secure Storage**: Store credentials securely and don't share them unnecessarily
- **Rotation**: Regularly rotate credentials for production storage backends

### Network Security

Consider network security for remote storage backends:

- **Encryption**: Use HTTPS for HTTP stores and encrypted connections for other backends
- **Firewalls**: Configure firewalls to restrict access to storage backends
- **VPNs**: Consider using VPNs for accessing sensitive storage backends

## Troubleshooting

### Connectivity Issues

If you can't connect to a storage backend:

1. **Network Connectivity**: Ensure you can reach the storage backend
   ```bash
   ping zdb.example.com
   ```

2. **Firewall Rules**: Check if firewalls are blocking access
   ```bash
   telnet zdb.example.com 9900
   ```

3. **Credentials**: Verify that your credentials are correct
   ```bash
   # For S3
   aws s3 ls s3://bucket-name --endpoint-url http://s3.example.com:9000
   ```

### Content Not Found

If content is missing from a storage backend:

1. **Hash Verification**: Verify that the content hash is correct
   ```bash
   # Calculate the hash of a file
   sha256sum file.txt
   ```

2. **Store Configuration**: Check if the storage backend is correctly configured
   ```bash
   # List stores in the flist
   rfs config -m output.fl store list
   ```

3. **Sharding Configuration**: Verify that the sharding configuration is correct
   ```bash
   # Check if the hash falls within the configured ranges
   echo $hash | cut -c1-2
   ```

## Next Steps

For more information about related concepts, see:
- [Understanding Flists](./flists.md)
- [Understanding Caching](./caching.md)
- [Understanding Sharding](./sharding.md)

For practical guides on working with storage backends, see the [Tutorials](../tutorials/) section.