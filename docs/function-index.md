# Function Index — Pack Flow

Scope: This edition covers only the pack pipeline and closely related helpers in the following files: [src/pack.rs](src/pack.rs), [src/tree_visitor.rs](src/tree_visitor.rs), [src/merge.rs](src/merge.rs), [src/upload.rs](src/upload.rs), [src/sync.rs](src/sync.rs), [src/exist.rs](src/exist.rs), [src/unpack.rs](src/unpack.rs).

Contents
- [src/pack.rs](#file-srcpackrs)
- [src/tree_visitor.rs](#file-srctree_visitorrs)
- [src/merge.rs](#file-srcmergers)
- [src/upload.rs](#file-srcuploadrs)
- [src/sync.rs](#file-srcsyncrs)
- [src/exist.rs](#file-srcexistrs)
- [src/unpack.rs](#file-srcunpackrs)

## File: [src/pack.rs](src/pack.rs)
<a id="file-srcpackrs"></a>

Public functions
- fn pack<P: Into<PathBuf>, S: Store>(writer, store, root, strip_password, sender) -> Result<()> — Builds routing from store, traverses the source tree iteratively, writes inodes, and enqueues file uploads; reports aggregated failures.

Internal helpers
- fn pack_one<S: Store>(list, writer, pool, Item(parent, path, name, meta), sender) -> Result<()> — Processes one directory: creates its inode, iterates entries to create children inodes, schedules file uploads, and queues subdirectories.
- fn Uploader<S>:   :new(store, writer, failures) -> Self — Initializes the concurrent uploader with shared block store, writer clone, and failure accumulator.
- async fn Uploader::upload(&mut self, ino, path: &Path) -> Result<()> — Reads the file in fixed-size chunks, stores each block, and records block IDs/keys in metadata.
- fn Uploader<S> as workers::Work::run(&mut self, (ino, path)) -> () — Worker entrypoint that uploads a file and records any error.

## File: [src/tree_visitor.rs](src/tree_visitor.rs)
<a id="file-srctree_visitorrs"></a>

Public functions
- fn TreeVisitor::new() -> Self — Constructs a visitor that prints a simple tree of inode entries.

Internal helpers
- fn TreeVisitor::print_entry(&self, path, node) -> () — Computes depth and prints a single tree line with an icon per file type.
- async fn TreeVisitor as WalkVisitor::visit(&mut self, path, node) -> Result<Walk> — Prints the entry and continues traversal.

## File: [src/merge.rs](src/merge.rs)
<a id="file-srcmergers"></a>

Public functions
- async fn merge(flist_path, server_url, token, target_flists, cache) -> Result<()> — Creates a new flist by walking target flists, reconstructing directory/file inodes into a fresh writer, copying block refs, backfilling missing blocks to the store, then uploading the resulting flist.

Internal helpers
- async fn MergeVisitor::ensure_parent_directory(&mut self, path: &Path) -> Result<u64> — Ensures parent directory inodes exist in the output, creating and caching them on demand.
- async fn MergeVisitor::copy_blocks(&mut self, source_ino: u64, dest_ino: u64) -> Result<()> — Copies block references into the writer and pushes missing blocks to the configured store via cache.
- async fn MergeVisitor as WalkVisitor::visit(&mut self, path, node) -> Result<Walk> — Mirrors directory and regular file nodes into the new flist, preserving metadata and wiring blocks.

## File: [src/upload.rs](src/upload.rs)
<a id="file-srcuploadrs"></a>

Public functions (pack-related)
- fn calculate_hash(data: &[u8]) -> String — Computes a 32-byte BLAKE2b digest and returns it as lowercase hex; used as per-block content ID.
- async fn split_file_into_blocks(file_path: &Path, block_size: usize) -> Result<(Vec<String>, Vec<(String, Vec<u8>)>)> — Reads the file into blocks, returning the list of block hashes and the paired (hash, data) buffers.
- fn calculate_file_hash(blocks: &[String]) -> String — Folds block hashes with SHA-256 to derive a deterministic file-level hash.
- async fn upload<P: AsRef<Path>>(file_path, server_url, block_size, token) -> Result<String> — Verifies missing blocks with the server, uploads only the missing ones in parallel, and returns the file hash.
- async fn upload_dir<P: AsRef<Path>>(dir_path, server_url, block_size, token, create_flist, flist_output) -> Result<()> — Recursively collects files; optionally creates a flist by invoking pack and then uploads that flist.

Internal helpers
- fn collect_files(dir_path: &Path, file_paths: &mut Vec<PathBuf>) -> std::io::Result<()> — Iterative DFS to gather all files under a directory.

Note: publish_website, get_token_from_server, and track are excluded here as they are outside the core pack pipeline.

## File: [src/sync.rs](src/sync.rs)
<a id="file-srcsyncrs"></a>

Public functions
- async fn sync(hash: Option<&str>, source_server, dest_server, token) -> Result<()> — Orchestrates either targeted sync of a single file by hash or a full-block sync between servers.
- async fn sync_all_blocks(source_server, dest_server, page_size, token) -> Result<()> — Pages through blocks on the source, checks existence on destination, and copies missing blocks with bounded concurrency.

Internal helpers
- async fn sync_blocks(file_hash: &str, source_server, dest_server, token) -> Result<()> — For a specific file hash, verifies missing blocks on destination and transfers them from source in parallel.

## File: [src/exist.rs](src/exist.rs)
<a id="file-srcexistrs"></a>

Public functions
- async fn exists<P: AsRef<Path>>(file_path, server_url, block_size) -> Result<()> — Splits a local file into blocks and checks their presence on a server concurrently; logs whether the whole file exists.
- async fn exists_by_hash(hash: String, server_url: String) -> Result<()> — Queries the server for blocks by file hash and reports existence without reading local data.

## File: [src/unpack.rs](src/unpack.rs)
<a id="file-srcunpackrs"></a>

Public functions
- async fn unpack<P: AsRef<Path>, S: Store>(meta: &Reader, cache: &Cache<S>, root, preserve) -> Result<()> — Walks metadata and reconstructs the filesystem to a root, downloading file blocks via cache; included here for symmetry with pack.

Internal helpers
- fn CopyVisitor::new(meta, cache, root, preserve) -> Self — Constructs a sequential copy visitor with optional ownership preservation.
- async fn CopyVisitor as WalkVisitor::visit(&mut self, path, node) -> Result<Walk> — Creates directories/files/symlinks, streams file blocks to disk, and applies mode/ownership when requested.
- fn ParallelCopyVisitor::new(meta, root, preserve, pool) -> Self — Configures a parallel variant that schedules file downloads to a worker pool.
- async fn ParallelCopyVisitor as WalkVisitor::visit(&mut self, path, node) -> Result<Walk> — Schedules regular files for background download; creates dirs and symlinks eagerly.
- fn Downloader::new(cache) -> Self — Initializes a downloader worker over a shared cache.
- async fn Downloader::download(&self, path: &Path, blocks: &[Block], mode: u32) -> Result<()> — Streams blocks to a new file at path and sets its permissions.
- fn Downloader<S> as workers::Work::run(&mut self, (path, blocks, mode)) -> () — Worker entrypoint that downloads a single file and logs errors.

# Function Index — Mount Flow

Scope: This section covers only the mount/FUSE and on-demand fetch path across the following files: [src/fs/mod.rs](src/fs/mod.rs), [src/lib.rs](src/lib.rs), [src/cache/mod.rs](src/cache/mod.rs), [src/store/mod.rs](src/store/mod.rs), [src/store/router.rs](src/store/router.rs).

Contents
- [src/fs/mod.rs](#file-srcfsmodrs)
- [src/lib.rs](#file-srclibrs)
- [src/cache/mod.rs](#file-srccachemodrs)
- [src/store/mod.rs](#file-srcstoremodrs)
- [src/store/router.rs](#file-srcstorerouterrs)

## File: [src/fs/mod.rs](src/fs/mod.rs)
<a id="file-srcfsmodrs"></a>

Public functions
- fn Filesystem<S>::new(meta: Reader, cache: Cache<S>) -> Self — Initializes a filesystem over metadata and a cache-backed store; configures an LRU for open block descriptors.
- async fn Filesystem<S>::mount<P: Into<PathBuf>>(&self, mnt: P) -> Result<()> — Mounts a FUSE session with polyfuse and dispatches requests to handlers (lookup, getattr, readdir, read, readlink, statfs); main entrypoint for the mount flow.

Internal helpers
- async fn Filesystem<S>::lookup(&self, req: &Request, op: op::Lookup<'_>) -> Result<()> — Resolves (parent, name) to an inode using metadata and replies with EntryOut and long TTLs; maps ENOENT when missing.
- async fn Filesystem<S>::getattr(&self, req: &Request, op: op::Getattr<'_>) -> Result<()> — Loads inode from metadata and fills FileAttr for the reply.
- async fn Filesystem<S>::readdir(&self, req: &Request, op: op::Readdir<'_>) -> Result<()> — Lists directory entries via paged metadata; injects . and .. and streams entries until the buffer is full.
- async fn Filesystem<S>::read(&self, req: &Request, op: op::Read<'_>) -> Result<()> — Serves file bytes lazily per block: computes chunk index, obtains a block fd from the LRU or cache.get (which downloads if needed), reads into the reply buffer, and requeues partial blocks into LRU.
- async fn Filesystem<S>::readlink(&self, req: &Request, op: op::Readlink<'_>) -> Result<()> — Returns symlink target from metadata or ENOLINK if absent or wrong type.
- async fn Filesystem<S>::statfs(&self, req: &Request, _op: op::Statfs<'_>) -> Result<()> — Replies filesystem stats (e.g., block size) required by the kernel.
- async fn AsyncSession::mount(mountpoint: PathBuf, config: KernelConfig) -> io::Result<Self> — Starts a polyfuse Session in a blocking task and wraps it in AsyncFd for async polling.
- async fn AsyncSession::next_request(&self) -> io::Result<Option<Request>> — Polls and yields the next FUSE request, handling WouldBlock readiness.
- fn Inode as AttributeFiller::fill(&self, attr: &mut FileAttr) -> () — Maps inode fields to FUSE attributes; used by getattr and lookup to populate replies.

## File: [src/lib.rs](src/lib.rs)
<a id="file-srclibrs"></a>

Public functions
- None — This module exposes crates and re-exports used by the mount pipeline but defines no mount-path functions.

## File: [src/cache/mod.rs](src/cache/mod.rs)
<a id="file-srccachemodrs"></a>

Public functions
- fn Cache<S>::new<P: Into<PathBuf>>(root: P, store: S) -> Self — Constructs the block cache rooted at a path over a block store; used to back Filesystem reads.
- async fn Cache<S>::get(&self, block: &Block) -> Result<(u64, File)> — Returns a cached block file and size, downloading from the store if the cache file is empty; uses per-file locking to serialize downloads and rewinds for reading.

Internal helpers
- async fn Cache<S>::download(&self, file: &mut File, block: &Block) -> Result<u64> — Fetches block bytes via the underlying store and writes them to the cache file; returns written size.
- async fn Cache<S>::prepare(&self, id: &[u8]) -> Result<File> — Ensures cache directories exist and opens the block file for read/write without truncation.
- fn Locker::new(f: &File) -> Locker — Wraps a file descriptor to manage advisory locks around block population.
- async fn Locker::lock(&self) -> Result<()> — Acquires an exclusive lock to prevent duplicate downloads of the same chunk.
- async fn Locker::unlock(&self) -> Result<()> — Releases the lock after the cache file is populated.

## File: [src/store/mod.rs](src/store/mod.rs)
<a id="file-srcstoremodrs"></a>

Public functions
- async fn get_router(meta: &fungi::Reader) -> Result<Router<Stores>> — Builds a multi-backend router from metadata routes; used when assembling the store stack for mount.
- async fn parse_router(urls: &[String]) -> anyhow::Result<Router<Stores>> — Parses range-qualified URLs into a router; alternative runtime configuration for mount.
- async fn make<U: AsRef<str>>(u: U) -> Result<Stores> — Instantiates a concrete backend from a URL; used by router builders during mount setup.

Trait and impl methods on the hot path
- async fn Store::get(&self, key: &[u8]) -> Result<Vec<u8>> — Abstract API to fetch a block by key; invoked by Cache::get during lazy reads.
- async fn Router<S> as Store::get(&self, key: &[u8]) -> Result<Vec<u8>> — Routes by first-byte prefix and queries matching stores in randomized order to balance load; aggregates errors if all backends fail.
- async fn Stores as Store::get(&self, key: &[u8]) -> Result<Vec<u8>> — Dispatches to the selected backend variant’s get implementation.

## File: [src/store/router.rs](src/store/router.rs)
<a id="file-srcstorerouterrs"></a>

Public functions
- fn Router<T>::new() -> Self — Creates an empty inclusive-range router keyed by the first byte of the content ID.
- fn Router<T>::add(&mut self, start: u8, end: u8, route: T) -> () — Registers a backend for an inclusive byte range to enable prefix-based routing.
- fn Router<T>::route(&self, i: u8) -> impl Iterator<Item = &T> — Returns all backends that cover the requested prefix; used by Router<S> Store::get to select candidates.