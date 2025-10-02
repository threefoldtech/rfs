# ADR-0003: Cache strategy and eviction policy

## Metadata
- ID: ADR-0003
- Title: Cache strategy and eviction policy
- Status: Proposed
- Date: 2025-10-02
- Deciders: RFS core maintainers
- Tags: cache, lru, eviction, integrity, concurrency, fuse
- Supersedes: None
- Superseded by: None

## Context
RFS mounts flists via FUSE with lazy, on-demand reads. A client-side cache reduces repeated backend fetches, caps read latency, and smooths throughput under bursty or concurrent access. The cache must respect filesystem semantics and operate within resource limits.

Constraints and expectations:
- POSIX read semantics where feasible: stable sizes/modes from metadata, consistent error mapping.
- Partial and random access: serve arbitrary offset/length without whole-file downloads.
- Concurrency: many files and blocks in parallel; avoid duplicate fetches for the same block.
- Resource limits: bounded disk/RAM; cache should not exhaust system storage.
- Network partitions and backend errors: misses may block or fail; cache must not be corrupted.
- Consistency: flists are content-addressed; cached blocks are safe to reuse across mounts; changes appear under new content hashes; no backend-driven invalidation.

Background: see [docs/concepts/caching.md](docs/concepts/caching.md) and mount flow in [docs/call-stacks/mount.md](docs/call-stacks/mount.md).

## Decision
Adopt a block-oriented, cache-first read path for the lazy-loading mount, with per-block single-writer concurrency control and a Proposed on-disk eviction policy. Details:

- Block alignment: content-addressed blocks referenced by flists; cache stores blocks by hash in a two-level hex path (Current; see [src/cache/mod.rs](src/cache/mod.rs)).
- Read path: FUSE read in [src/fs/mod.rs](src/fs/mod.rs) maps offset to block, checks in-memory LRU of open descriptors, then cache.get(); on miss, fetch via routing in [src/store/router.rs](src/store/router.rs) and backends in [src/store/mod.rs](src/store/mod.rs), write to cache, and reply (Current).
- Eviction: LRU with configurable capacity (bytes and/or entry count) with optional high/low watermarks to batch deletes (Proposed). Current behavior has no automatic on-disk eviction; manual cleanup per [docs/concepts/caching.md](docs/concepts/caching.md).
- Concurrency control: single-writer/multi-reader per block using an exclusive file lock around population to deduplicate in-flight downloads across tasks/processes (Current; see [src/cache/mod.rs](src/cache/mod.rs)).
- Persistence and layout: cache lives under a configurable path supplied by CLI; entries are safe to delete; cache is rebuilt on demand (Current; [src/main.rs](src/main.rs)).
- Integrity: validate blocks by content hash on write/read; purge and surface error on mismatch (Proposed; not verified in current code path).

## Rationale
- Improves repeated read performance and reduces backend load through local reuse.
- LRU approximates recency locality and is simple to implement and reason about; high/low watermarks cap disk usage and amortize deletions.
- Single-flight population prevents duplicate downloads, saving bandwidth and latency under concurrent readers.
- Block granularity aligns with flist metadata and supports partial/random access efficiently.

## Alternatives Considered
- LFU or ARC — better frequency sensitivity but higher complexity and metadata overhead.
- Write-through local mirror — simpler lookups but consumes full dataset storage; slower initial mount.
- Whole-file prefetch — good for sequential workloads; wastes bandwidth/storage for sparse/random access.
- Rely only on OS page cache — no persistence across mounts/processes; no offline reuse.
- TTL-based expiry without recency — predictable churn but discards hot data prematurely.
- RAM-only caching — fastest but limited capacity; volatile across restarts.

## Consequences
Positive:
- Reduced latency after warm-up and fewer backend operations.
- Bounded disk usage once eviction is implemented.
- Predictable behavior under concurrency with inflight deduplication.

Negative and trade-offs:
- Cold misses incur backend latency.
- Eviction design and implementation add complexity and potential edge cases.
- Disk space consumption by the cache; checksum validation adds CPU and IO overhead if enabled.

## Implementation Notes
High-level flow:
- [src/fs/mod.rs](src/fs/mod.rs) read ➜ [src/cache/mod.rs](src/cache/mod.rs) lookup ➜ miss ➜ [src/store/router.rs](src/store/router.rs) ➜ [src/store/mod.rs](src/store/mod.rs) ➜ write to cache ➜ reply. See [docs/call-stacks/mount.md](docs/call-stacks/mount.md) and [docs/function-index.md](docs/function-index.md).

Configuration knobs (documented and validated; names/values not fixed here):
- Cache directory path.
- Capacity limits: max bytes and/or entry count (Proposed).
- Watermarks: high/low thresholds for background eviction (Proposed).
- Concurrency: bounds for parallel fetches; single-flight per block is mandatory.
- Integrity checks: enable/disable content-hash verification (Proposed).

Current implementation notes:
- On-disk layout uses hex-encoded block id under two nested directories to avoid large fanout (Current; [src/cache/mod.rs](src/cache/mod.rs)).
- An in-memory LRU of open block descriptors exists in the filesystem layer to reduce fd churn; it is not an on-disk eviction policy (Current; [src/fs/mod.rs](src/fs/mod.rs)).

## Security and Privacy
- Integrity derives from content-addressed design; enabling verification mitigates corruption (Proposed where not present).
- Ensure no credentials are ever written into cached payloads; backends handle secrets and routing (Current practice).
- Recommend restrictive permissions on the cache directory to prevent unauthorized reads of cached data.

## Operational Considerations
- Metrics: cache hit/miss rates, bytes served from cache vs backend, eviction counts, fetch latencies, and error rates.
- Logging: summarize backend selection and retries; log integrity failures with block ids.
- Maintenance: provide a safe cleanup command; define/track cache format version for future changes (Proposed).
- Corruption handling: on read/verify failure, purge block and refetch; bubble up EIO on persistent failure.

## Performance and Scaling
- Block size affects hit rate and read amplification; 512 KiB chunks are currently used in the read path (Current).
- Sequential access can benefit from optional prefetch windows (Proposed); random access relies on cache hits and single-block misses.
- Under many readers, ensure lock contention is limited to the first miss per block; consider background prefetch for adjacent blocks (Proposed).

## Open Questions
- Exact eviction triggers and data structures (pure LRU vs segmented/admission policies).
- Prefetch heuristics and controls; when to enable and how to bound bandwidth.
- Cache format versioning and migration strategy.
- Potential for distributed/shared cache across hosts.
- QoS/backpressure when local storage nears capacity; how to prioritize evictions.

## References
- [docs/concepts/caching.md](docs/concepts/caching.md)
- [docs/call-stacks/mount.md](docs/call-stacks/mount.md)
- [docs/architecture/adr/ADR-0002-fuse-lazy-loading.md](docs/architecture/adr/ADR-0002-fuse-lazy-loading.md)
- [src/cache/mod.rs](src/cache/mod.rs)
- [src/fs/mod.rs](src/fs/mod.rs)
- [src/store/router.rs](src/store/router.rs)
- [src/store/mod.rs](src/store/mod.rs)

## Changelog
- 2025-10-02 — Created