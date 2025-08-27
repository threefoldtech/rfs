# RFS Storage Backends

This document provides detailed information about the storage backends supported by RFS and how they are used to store and retrieve file content.

## Overview

RFS separates file metadata (stored in flists) from file content (stored in backends). This separation allows for efficient distribution of filesystem structures while minimizing data transfer. The actual file content is stored in configurable storage backends, which can be local or remote.

## Supported Storage Backends

RFS currently supports the following storage backends:

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

## Sharding and Replication

RFS supports sharding and replication of content across multiple storage backends.

### Sharding

Sharding distributes content across multiple storage backends based on the content hash.

#### Format
```
xx-yy=store_url
```

Where:
- `xx` is the start of the hash range (in hex)
- `yy` is the end of the hash range (in hex)
- `store_url` is the URL of the storage backend

#### Example
```
00-80=dir:///tmp/store0 81-ff=dir:///tmp/store1
```

This configuration stores content with hashes starting with `00` through `80` in `/tmp/store0` and content with hashes starting with `81` through `ff` in `/tmp/store1`.

### Replication

Replication stores the same content in multiple storage backends for redundancy.

#### Implementation
To replicate content, specify the same hash range for multiple storage backends:

```
00-ff=dir:///tmp/store1 00-ff=dir:///tmp/store2
```

This configuration stores all content in both `/tmp/store1` and `/tmp/store2`.

## Store Router

The Store Router is a component that manages the routing of content to and from the appropriate storage backends based on the content hash.

### Functionality

- **Store Selection**: Determines which storage backend(s) to use for a given content hash
- **Read Strategy**: Implements strategies for reading from multiple backends (e.g., try first available)
- **Write Strategy**: Implements strategies for writing to multiple backends (e.g., write to all)
- **Error Handling**: Manages errors and retries when interacting with storage backends

## Configuration in Flists

Storage backend information is stored in the flist metadata. This allows the flist to be self-contained, with all the information needed to access the content.

### Password Stripping

By default, passwords in storage URLs are stripped when creating an flist to prevent unauthorized write access. This behavior can be disabled with the `--no-strip-password` flag.

## Best Practices

1. **Local Development**: Use the directory store for local development and testing.

2. **Production Deployments**: Use ZDB or S3 for production deployments.

3. **Content Distribution**: Use HTTP for wide distribution of content.

4. **Critical Data**: Use replication across multiple storage backends for critical data.

5. **Performance**: Use sharding across multiple storage backends for improved performance with large datasets.

6. **Security**: Configure storage backends with appropriate access controls:
   - Read-only access for public content
   - Write access protected by authentication
   - Consider using separate credentials for reading and writing

## Future Storage Backends

The RFS architecture is designed to be extensible, allowing for the addition of new storage backends. Potential future backends include:

- **IPFS**: For decentralized content-addressed storage
- **WebDAV**: For integration with existing WebDAV servers
- **FTP/SFTP**: For integration with existing FTP servers
- **Cloud-specific backends**: For optimized integration with specific cloud providers

## Next Steps

For information on how to use these storage backends in practice, refer to the [User Guides](../user-guides/) section.