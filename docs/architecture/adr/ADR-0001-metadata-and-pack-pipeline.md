# ADR-0001: Metadata layout and pack pipeline

## Metadata
- ID: ADR-0001
- Title: Metadata layout and pack pipeline
- Status: Proposed
- Date: 2025-10-02
- Deciders: RFS core maintainers
- Tags: flist, metadata, pack, routing, deduplication
- Supersedes: None
- Superseded by: None

## Context
RFS packages a filesystem into a compact flist that contains only metadata and content references while storing file data as content-addressed blocks in one or more backends. The goals for the metadata layout and pack pipeline are:

- Represent a complete filesystem with inodes for directories, regular files, symlinks, and device-like entries where applicable.
- For regular files, record per-file ordered block references derived from chunked content.
- Define routing to storage backends inside the flist so block retrieval and writes can be directed by hash prefix ranges.
- Enable cross-flist deduplication and reproducible packaging runs.
- Keep credentials out of on-disk metadata; allow redaction while preserving route shape.

This ADR captures the current design and rationale behind metadata layout and the pack pipeline that builds it. See background material in [docs/concepts/flists.md](docs/concepts/flists.md) and the pack call stack in [docs/call-stacks/pack.md](docs/call-stacks/pack.md).

## Decision
The pack pipeline produces an flist with content-addressed block references for files and embeds route mappings to storage backends keyed by hash/prefix ranges. During packing, files are chunked, block digests are computed, server-side existence is checked, only missing blocks are uploaded, and finally the flist is written for downstream use (merge, mount, or unpack). Credentials in routes are stripped before metadata is persisted to disk.

## Rationale
- Deduplication: Content-addressed blocks allow identical chunks across files and runs to be stored once, significantly reducing storage.
- Resumability and idempotence: Because block identity is derived from content, interrupted or repeated runs can skip already present data.
- Network efficiency: An existence check before upload avoids transferring blocks that the server already has.
- Portability: Embedding filename-linked routes in metadata allows downstream consumers to reconstruct a router without external configuration.
- Separation of concerns: Flists hold structure and references; backends hold bytes. This keeps metadata small and easy to distribute.

## Alternatives Considered
- Monolithic archives (tar): Simpler, but no block-level dedup, no lazy fetch, and requires full extraction to access content.
- Per-file whole-object storage: Deduplicates only identical whole files and performs poorly with minor edits in large files.
- Single-backend without routing: Simpler configuration but limits scalability, redundancy, and locality-aware reads/writes.
- Merge sequencing: Pre-upload merge vs post-upload merge; current approach allows post-upload merge using existing block references in [src/merge.rs](src/merge.rs).
- Prefetch-oriented design: Optimizes first-run latency but forfeits lazy/on-demand fetch benefits used by mount.

## Consequences
Positive:
- High dedup and efficient incremental updates across runs and sources.
- Stable, reproducible outputs driven by content hashing and deterministic traversal.
- Flexible scaling by adding routes/backends without changing the flist structure.

Negative/trade-offs:
- Additional complexity in maintaining routing tables and index structures.
- Tight consistency required between client packer and server implementations for exist/upload semantics.
- Metadata versions must evolve carefully to preserve compatibility guarantees.

## Implementation Notes
High-level pack flow and key modules:
- Entry: CLI triggers pack in [src/pack.rs](src/pack.rs). It initializes store routing and the flist writer, then iteratively walks the source tree.
- Tree traversal: Directory entries are discovered and inodes created via helpers patterned after [src/tree_visitor.rs](src/tree_visitor.rs).
- Chunking and hashing: For each regular file, content is split into blocks and a cryptographic hash is computed per block; a file-level hash may be derived from the ordered block list. See helpers documented in [docs/function-index.md](docs/function-index.md) and [src/upload.rs](src/upload.rs).
- Existence check: Before upload, blocks are checked against configured server(s) to avoid duplicates, using routines described in [src/exist.rs](src/exist.rs).
- Upload: Missing blocks are uploaded, typically in parallel, and block references are recorded back into the writer in [src/pack.rs](src/pack.rs) with helper routines in [src/upload.rs](src/upload.rs).
- Routing: Route mappings to backends (e.g., by first-byte prefix) are embedded into the flist; at runtime a router similar to [src/store/router.rs](src/store/router.rs) is reconstructed.
- Emit metadata: The finalized flist is written for downstream consumption. Related workflows include merge in [src/merge.rs](src/merge.rs) and unpack in [src/unpack.rs](src/unpack.rs).

Additional references: the sequence diagram and steps in [docs/call-stacks/pack.md](docs/call-stacks/pack.md) and the function summaries in [docs/function-index.md](docs/function-index.md).

## Security and Privacy
- Integrity: Content hashes enable verification that retrieved blocks match what was packed.
- Secret redaction: When persisting routes in the flist, credentials must be removed from URLs while keeping topology and ranges.
- Transport/auth: Authentication, authorization, and encryption are provided by storage backends and servers configured via routes; flist does not carry live credentials.
- Tampering: If metadata is modified, consumers should validate block hashes upon retrieval and handle mismatches as integrity failures.

## Operational Considerations
- Observability: Provide progress for traversal and uploads, track retries, and surface a failure list for any files that could not be processed.
- Idempotent uploads: Re-running pack should be safe and skip already present blocks; existence probes should be tolerant to transient errors.
- Parallelism: Bound concurrency for CPU (hashing) and IO (upload) separately to balance throughput and resource use.
- Failure handling: Partial uploads should not corrupt metadata; failures are aggregated and reported, and the flist should only be finalized when consistent.
- Reproducibility: Deterministic traversal order and stable hashing yield reproducible flists given identical inputs and configuration.

## Performance and Scaling
- Chunk size: Larger chunks reduce index size and exist round trips but may reduce dedup granularity; smaller chunks increase overhead but improve dedup.
- Concurrency: Tune hashing and upload worker counts to saturate CPU and network without overwhelming backends.
- Exist checks: Batched or pipelined existence queries reduce round trips; caching positive results per run can avoid repeats.
- Backend fan-out: Router-based sharding spreads load and allows parallel uploads/downloads across backends; ordering and retries should avoid hot spots.

## Open Questions
- Metadata versioning policy and migration strategy across releases.
- Hash algorithm and version upgrades; signaling and mixed-version interoperability.
- Atomic flist finalization to avoid readers observing partial state.
- Cross-compatibility with older servers and features gating for newer fields.
- Consistency guarantees when routing to multiple stores (write quorum, read fallback).

## References
- [docs/concepts/flists.md](docs/concepts/flists.md)
- [docs/call-stacks/pack.md](docs/call-stacks/pack.md)
- [docs/function-index.md](docs/function-index.md)
- [src/pack.rs](src/pack.rs)
- [src/tree_visitor.rs](src/tree_visitor.rs)
- [src/exist.rs](src/exist.rs)
- [src/upload.rs](src/upload.rs)
- [src/merge.rs](src/merge.rs)
- [src/unpack.rs](src/unpack.rs)
- [src/store/router.rs](src/store/router.rs)

## Changelog
- 2025-10-02 — Created