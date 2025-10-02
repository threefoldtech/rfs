# Basics of rfs

rfs packages a filesystem into an flist that holds only metadata—inode tree, per-file block references, and store routing—while file bytes live as content-addressed blocks in remote/local stores.
Two primary flows shape the system: pack builds the flist and uploads any missing blocks; mount exposes the flist through FUSE, serving bytes lazily.
A cache captures fetched blocks on disk for reuse, and a store router maps block prefixes to backends for reads and writes.
Use this page to get oriented, then follow the links to detailed call stacks, entrypoints, and ADRs.

## Core Flows

Pack — summary and link:
- Walk source tree, chunk and hash file content, and record block references in metadata.
- Check store existence and upload only missing blocks; embed routing info in the flist.
- Produce a deterministic flist usable by merge, mount, and unpack.
- See the full call stack in [docs/call-stacks/pack.md](docs/call-stacks/pack.md).

Mount — summary and link:
- Initialize a FUSE filesystem over an flist with a metadata reader and on-disk block cache.
- Serve lookups, getattr, and readdir from metadata; serve read by block index and offset.
- On cache miss, route to a backend, fetch the block, write it to cache, then reply.
- Maintain an LRU of open block descriptors to accelerate repeated reads.
- See the full call stack in [docs/call-stacks/mount.md](docs/call-stacks/mount.md).

## Quick CLI Anchors

- High-level CLI entrypoints: [docs/entrypoints.md](docs/entrypoints.md)
- Function inventory: [docs/function-index.md](docs/function-index.md)

## Key Design Decisions

- Metadata and pack pipeline — flist structure, chunking, existence checks, and route embedding: [docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md](docs/architecture/adr/ADR-0001-metadata-and-pack-pipeline.md)
- FUSE lazy-loading — serving file data on demand via cache and backends: [docs/architecture/adr/ADR-0002-fuse-lazy-loading.md](docs/architecture/adr/ADR-0002-fuse-lazy-loading.md)
- Cache strategy — layout, eviction, and concurrency: [docs/architecture/adr/ADR-0003-cache-strategy.md](docs/architecture/adr/ADR-0003-cache-strategy.md)
- Storage backends and routing — prefix-based sharding and fallback: [docs/architecture/adr/ADR-0004-storage-backends-routing.md](docs/architecture/adr/ADR-0004-storage-backends-routing.md)
- Server API and block serving — exist, get, and upload semantics: [docs/architecture/adr/ADR-0005-server-api-and-block-serving.md](docs/architecture/adr/ADR-0005-server-api-and-block-serving.md)
- Browse all ADRs in the index: [docs/architecture/adr/index.md](docs/architecture/adr/index.md)

## Next steps

- Read the architecture overview: [docs/architecture/overview.md](docs/architecture/overview.md)
- Check documentation conventions before adding or editing docs: [docs/docs-conventions.md](docs/docs-conventions.md)