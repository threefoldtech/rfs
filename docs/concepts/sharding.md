# Understanding Sharding and Replication in RFS

This document explains the concepts of sharding and replication in the RFS ecosystem, how they work, and how to implement them effectively.

## What are Sharding and Replication?

In RFS, sharding and replication are strategies for distributing content across multiple storage backends:

- **Sharding**: Distributing different content across multiple storage backends based on content hashes
- **Replication**: Storing the same content in multiple storage backends for redundancy

These strategies can be used independently or together to achieve different goals.

## Sharding

### How Sharding Works

Sharding in RFS works by partitioning the hash space and assigning different ranges to different storage backends:

1. **Hash Calculation**: When a file is processed, its content is hashed (currently using SHA-256).

2. **Range Assignment**: The hash is compared against configured ranges to determine which storage backend(s) should store the content.

3. **Content Distribution**: The content is then stored in the appropriate backend(s) based on the hash.

### Sharding Syntax

Sharding is configured using the following syntax:

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

This configuration:
- Stores content with hashes starting with `00` through `7f` in `/tmp/store1`
- Stores content with hashes starting with `80` through `ff` in `/tmp/store2`

### Benefits of Sharding

Sharding offers several benefits:

1. **Scalability**: Distributes the storage load across multiple backends
2. **Performance**: Enables parallel operations across multiple backends
3. **Capacity**: Allows for larger total storage capacity than a single backend
4. **Isolation**: Can isolate different types of content or workloads

### Sharding Strategies

#### Even Distribution

For even distribution of content, divide the hash space into equal parts:

```bash
# Two equal shards
rfs pack -m output.fl \
  -s 00-7f=dir:///tmp/store1 \
  -s 80-ff=dir:///tmp/store2 \
  /path/to/directory

# Four equal shards
rfs pack -m output.fl \
  -s 00-3f=dir:///tmp/store1 \
  -s 40-7f=dir:///tmp/store2 \
  -s 80-bf=dir:///tmp/store3 \
  -s c0-ff=dir:///tmp/store4 \
  /path/to/directory
```

#### Backend-Specific Sharding

You can use different backend types for different ranges:

```bash
# Fast local store for frequently accessed content, remote store for the rest
rfs pack -m output.fl \
  -s 00-1f=dir:///mnt/ssd/store \
  -s 20-ff=zdb://zdb.example.com:9900/namespace \
  /path/to/directory
```

#### Geographic Sharding

For globally distributed applications, shard by geographic region:

```bash
# US store for US users, EU store for EU users
rfs pack -m output.fl \
  -s 00-7f=s3://us-east-1.amazonaws.com/us-bucket \
  -s 80-ff=s3://eu-west-1.amazonaws.com/eu-bucket \
  /path/to/directory
```

## Replication

### How Replication Works

Replication in RFS works by storing the same content in multiple storage backends:

1. **Multiple Assignments**: The same hash range is assigned to multiple storage backends.

2. **Parallel Storage**: When content is stored, it is written to all matching backends.

3. **Fallback Retrieval**: When content is retrieved, RFS tries each backend in order until the content is found.

### Replication Syntax

Replication is configured by specifying the same hash range for multiple storage backends:

```
xx-yy=store_url1 xx-yy=store_url2
```

For example:
```
00-ff=dir:///tmp/store1 00-ff=dir:///tmp/store2
```

This configuration stores all content in both `/tmp/store1` and `/tmp/store2`.

### Benefits of Replication

Replication offers several benefits:

1. **Redundancy**: Protects against backend failures
2. **Availability**: Improves content availability
3. **Read Performance**: Enables reading from the fastest available backend
4. **Geographic Distribution**: Allows content to be available in multiple locations

### Replication Strategies

#### Full Replication

Replicate all content across all backends:

```bash
# Full replication across two backends
rfs pack -m output.fl \
  -s dir:///tmp/store1 \
  -s dir:///tmp/store2 \
  /path/to/directory
```

#### Partial Replication

Replicate only certain ranges of content:

```bash
# Replicate critical content (first 16 hash values), shard the rest
rfs pack -m output.fl \
  -s 00-0f=dir:///tmp/store1 \
  -s 00-0f=dir:///tmp/store2 \
  -s 10-7f=dir:///tmp/store1 \
  -s 80-ff=dir:///tmp/store2 \
  /path/to/directory
```

#### Tiered Replication

Replicate across different types of backends:

```bash
# Replicate across local and remote storage
rfs pack -m output.fl \
  -s dir:///tmp/local-store \
  -s zdb://zdb.example.com:9900/namespace \
  /path/to/directory
```

## Combining Sharding and Replication

Sharding and replication can be combined to create sophisticated storage strategies:

### Example: Sharded Replication

```bash
# Shard across two pairs of replicated backends
rfs pack -m output.fl \
  -s 00-7f=dir:///tmp/store1a \
  -s 00-7f=dir:///tmp/store1b \
  -s 80-ff=dir:///tmp/store2a \
  -s 80-ff=dir:///tmp/store2b \
  /path/to/directory
```

This configuration:
- Shards content into two ranges (`00-7f` and `80-ff`)
- Replicates each range across two backends
- Results in four storage backends total

### Example: Replicated Sharding

```bash
# Replicate across two sharded systems
rfs pack -m output.fl \
  -s 00-7f=dir:///tmp/system1/store1 \
  -s 80-ff=dir:///tmp/system1/store2 \
  -s 00-7f=dir:///tmp/system2/store1 \
  -s 80-ff=dir:///tmp/system2/store2 \
  /path/to/directory
```

This configuration:
- Creates two complete sharded systems
- Replicates all content across both systems
- Provides both sharding and replication benefits

## Implementation Considerations

### Hash Distribution

The distribution of content across shards depends on the hash function's properties:

- **Uniformity**: SHA-256 produces uniformly distributed hashes, so equal ranges should receive roughly equal amounts of content
- **Determinism**: The same content always produces the same hash, ensuring consistent shard assignment
- **Prefix Selection**: RFS uses the first bytes of the hash for sharding, which works well for uniformly distributed hashes

### Performance Impact

Sharding and replication affect performance in different ways:

- **Write Performance**: 
  - Sharding can improve write performance through parallelization
  - Replication typically reduces write performance as content must be written to multiple backends

- **Read Performance**:
  - Sharding can improve read performance for parallel access patterns
  - Replication can improve read performance by enabling access from the fastest available backend

### Storage Efficiency

Consider the storage efficiency implications:

- **Sharding**: Distributes storage requirements across backends without increasing total storage needs
- **Replication**: Increases total storage requirements proportionally to the replication factor

### Failure Handling

Different configurations handle failures differently:

- **Sharding without Replication**: If a backend fails, content in that shard becomes unavailable
- **Replication without Sharding**: If a backend fails, all content remains available from other backends
- **Sharded Replication**: If a backend fails, only the content in that shard needs to be served from replicas

## Best Practices

### 1. Balance Sharding and Replication

Choose an appropriate balance between sharding and replication based on your requirements:

- **High Availability**: Prioritize replication
- **Scalability**: Prioritize sharding
- **Both**: Implement sharded replication

### 2. Consider Content Distribution

Analyze your content distribution to determine optimal sharding strategies:

- **Even Distribution**: For unknown or uniform content patterns
- **Content-Aware**: For known content patterns with specific access characteristics

### 3. Monitor Backend Health

Regularly monitor the health of your storage backends:

- **Availability**: Ensure backends are accessible
- **Capacity**: Monitor storage usage
- **Performance**: Track response times

### 4. Plan for Recovery

Develop recovery procedures for backend failures:

- **Replication**: Ensure content is replicated before a backend is decommissioned
- **Resharding**: Plan for redistributing content if sharding configuration changes
- **Backup**: Maintain backups of critical content

### 5. Document Your Configuration

Document your sharding and replication configuration:

- **Range Assignments**: Document which ranges are assigned to which backends
- **Replication Factors**: Document how many copies of each range exist
- **Recovery Procedures**: Document how to recover from backend failures

## Examples

### Basic Sharding Example

```bash
# Create two store directories
mkdir -p /tmp/store1 /tmp/store2

# Pack a directory with sharding
rfs pack -m sharded.fl \
  -s 00-7f=dir:///tmp/store1 \
  -s 80-ff=dir:///tmp/store2 \
  /path/to/directory

# Verify the distribution
du -sh /tmp/store1 /tmp/store2
```

### Basic Replication Example

```bash
# Create two store directories
mkdir -p /tmp/store1 /tmp/store2

# Pack a directory with replication
rfs pack -m replicated.fl \
  -s dir:///tmp/store1 \
  -s dir:///tmp/store2 \
  /path/to/directory

# Verify the replication
du -sh /tmp/store1 /tmp/store2
```

### Advanced Configuration Example

```bash
# Create four store directories
mkdir -p /tmp/system1/store1 /tmp/system1/store2 /tmp/system2/store1 /tmp/system2/store2

# Pack a directory with sharded replication
rfs pack -m advanced.fl \
  -s 00-7f=dir:///tmp/system1/store1 \
  -s 80-ff=dir:///tmp/system1/store2 \
  -s 00-7f=dir:///tmp/system2/store1 \
  -s 80-ff=dir:///tmp/system2/store2 \
  /path/to/directory

# Verify the distribution
du -sh /tmp/system1/store1 /tmp/system1/store2 /tmp/system2/store1 /tmp/system2/store2
```

## Next Steps

For more information about related concepts, see:
- [Understanding Flists](./flists.md)
- [Understanding Storage Backends](./stores.md)
- [Understanding Caching](./caching.md)

For practical guides on implementing sharding and replication, see the [Tutorials](../tutorials/) section.