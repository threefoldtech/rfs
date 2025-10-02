# ADR-0002: FUSE lazy-loading and on-demand fetch

## Metadata
- ID: ADR-0002
- Title: FUSE lazy-loading and on-demand fetch
- Status: Proposed
- Date: 2025-10-02
- Deciders: RFS core maintainers
- Tags: fuse, lazy-loading, cache, routing, backends
- Supersedes: None
- Superseded by: None

## Context
RFS mounts an flist as a POSIX-like filesystem using FUSE. The goal is to expose the full directory tree immediately while downloading file bytes only when accessed, minimizing upfront IO, storage, and latency to first mount. Reads should retrieve exactly the needed ranges and reuse fetched data efficiently.

Constraints and expectations:
- POSIX semantics where feasible: stable inodes, lookup/getattr/readdir behavior, consistent file sizes and modes derived from metadata.
- Partial reads: map arbitrary offset and size requests to content-addressed blocks without requiring whole-file downloads.
- Concurrency: serve multiple concurrent reads across files and within the same file; serialize cache population per block to avoid duplicate downloads.
- Offline and latency behavior: when backends are unavailable, operations should fail predictably without corrupting cache or metadata; reads may block until fetch or error.
- Error mapping: missing blocks or integrity failures must surface consistent errors (e.g., IO errors such as EIO or ENOENT as appropriate) to callers; errors should not leak backend-specific details.

Background materials: see the mount flow in [docs/call-stacks/mount.md](docs/call-stacks/mount.md) and function summaries in [docs/function-index.md](docs/function-index.md).

## Decision
Expose flists via a FUSE filesystem that serves directory and inode metadata directly from the flist and fetches file data lazily on read by translating read offsets to block indices, using a cache-first policy that on miss routes retrieval by hash or prefix through a store router to concrete backends, then fills the cache and serves the kernel response.

## Rationale
- Minimize upfront work: mounting becomes fast and constant-time relative to tree size because only metadata is loaded.
- Pay-as-you-go IO: only the bytes an application touches are fetched, which reduces bandwidth and storage for large, sparse, or exploratory workloads.
- Amortize via caching: repeated reads benefit from local cache, improving throughput and reducing backend load.
- Portability and pluggability: routing by content hash/prefix keeps backend selection independent of mount logic; backends encapsulate auth and transport.
- Robustness: cache-first with controlled retries degrades gracefully under transient failures while preserving integrity guarantees.

## Alternatives Considered
- Whole-file prefetch at open — simpler mental model and predictable latency after open, but wastes bandwidth and storage for large files accessed sparsely.
- Full pre-mount download — fastest reads after mount, but long mount times and high storage usage; unsuitable for large datasets.
- Userspace NFS or HTTP-backed FS — shifts semantics and dependencies; harder to leverage content-addressed routing and existing flists.
- Union or overlay with a local copy — complicates consistency and doubles storage for modified paths.
- Eager prefetch windows — improves sequential throughput but risks fetching unused data; can be added later as a heuristic.
- Per-process page cache only — relies solely on kernel caching without a persistent or shareable block cache; misses cross-process reuse and offline benefits.

## Consequences
Positive:
- Minimal local storage and fast mount independent of dataset size.
- Suited to large, read-mostly datasets and exploratory access patterns.
- Cache accelerates re-reads and enables limited offline access for previously fetched blocks.

Negative and trade-offs:
- First access incurs network latency; random small reads may be slower without prefetch.
- Cache eviction and invalidation policy increase complexity.
- Routing across backends introduces additional failure modes and error mapping concerns.
- Requires careful adherence to consistent error semantics from FUSE to applications.

## Implementation Notes
High-level call path (see [docs/call-stacks/mount.md](docs/call-stacks/mount.md)):
- Entry: CLI mount in [src/main.rs](src/main.rs) initializes metadata reader, cache, and routes, then starts FUSE.
- Filesystem ops in [src/fs/mod.rs](src/fs/mod.rs): handle open, lookup/getattr, readdir, and read using flist metadata.
- Cache in [src/cache/mod.rs](src/cache/mod.rs): cache-first lookup of content-addressed blocks; on miss, coordinate fetch and populate before serving.
- Router in [src/store/router.rs](src/store/router.rs): map block id or prefix to a backend.
- Backends in [src/store/mod.rs](src/store/mod.rs): perform network IO, verify integrity, and return bytes.
- After fetch, write block into cache and satisfy the pending read back to the kernel.

Concurrency:
- Serialize cache fill per block with a lock or single-flight guard to prevent duplicate downloads.
- Allow parallel reads across distinct blocks and files; bound concurrency to avoid overwhelming backends.

Error handling:
- Map integrity failures or missing blocks to IO-layer errors (e.g., EIO) and nonexistent paths to ENOENT.
- Avoid exposing backend-specific status codes; log details internally for diagnostics.

Resiliency:
- Apply bounded retries with backoff for transient network failures.
- Do not persist partial or corrupt blocks to the cache; verify content against expected hashes.

## Security and Privacy
- Integrity: content-addressed blocks allow verification that fetched data matches expected hashes from metadata.
- Authentication and authorization: handled by storage backends; credentials are not persisted in the flist.
- Credential hygiene: router reconstruction from flist uses redacted routes; secrets are supplied via runtime configuration and not written back.
- Path traversal and symlinks: resolve using flist metadata; avoid following untrusted external paths that could escape the mount root.

## Operational Considerations
- Observability: instrument cache hits/misses, fetch counts, latencies, and error rates; log backend selection and retries at summarized levels.
- Tuning knobs: cache size and eviction policy, read concurrency, retry limits, and optional prefetch heuristics.
- Network partitions: operations degrade to cache-only reads; new blocks fail fast with clear errors.
- Graceful unmount: drain in-flight fetches or cancel cleanly; flush cache metadata as needed.

## Performance and Scaling
- Block size: larger blocks reduce routing and metadata overhead but increase first-byte latency and waste for small reads; smaller blocks do the opposite.
- Access patterns: sequential reads can benefit from read-ahead; random reads rely on cache and direct misses to backends.
- Prefetch heuristics: optional windowed prefetch may be enabled for high sequentiality once validated.
- Store fan-out: routing by prefix enables parallel fetch across multiple backends; avoid hot spots via balanced prefix ranges.
- Cache hit rate: dominates end-to-end performance after warmup; monitor and tune cache policy accordingly.
- Kernel page size interaction: align block sizes and read buffers to reduce copy and syscall overhead where possible.

## Open Questions
- Cache eviction policy specifics (LRU variants, size ceilings, admission).
- Read-ahead and prefetch strategy and controls.
- Backpressure on concurrent fetches to protect backends and the network.
- Consistency and fallback when the same block is present in multiple stores.
- Offline semantics for partially populated files.
- Metadata format versioning and compatibility with evolving routes or hashing.

## References
- [docs/function-index.md](docs/function-index.md)
- [docs/call-stacks/mount.md](docs/call-stacks/mount.md)
- [src/main.rs](src/main.rs)
- [src/fs/mod.rs](src/fs/mod.rs)
- [src/cache/mod.rs](src/cache/mod.rs)
- [src/store/router.rs](src/store/router.rs)
- [src/store/mod.rs](src/store/mod.rs)

## Changelog
- 2025-10-02 — Created