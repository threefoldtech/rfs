# CLI Entrypoints and Execution Paths

This page inventories the CLI commands and subcommands defined in [src/main.rs](src/main.rs) and summarizes how they map to high-level execution paths. Mappings are derived strictly from [src/main.rs](src/main.rs) and [src/server_api.rs](src/server_api.rs). For each entrypoint we list key options that materially affect the path, a primary call-path sequence using filename links only, and a cross-reference to [docs/function-index.md](docs/function-index.md).

Contents

- [Command: mount](#cmd-mount)
- [Command: pack](#cmd-pack)
- [Command: unpack](#cmd-unpack)
- [Command: clone](#cmd-clone)
- [Command: config](#cmd-config)
- [Command: merge](#cmd-merge)
- [Command: docker](#cmd-docker)
- [Command: server](#cmd-server)
- [Command: upload](#cmd-upload)
- [Command: upload-dir](#cmd-upload-dir)
- [Command: exists](#cmd-exists)
- [Command: download](#cmd-download)
- [Command: download-dir](#cmd-download-dir)
- [Command: flist-create](#cmd-flist-create)
- [Command: website-publish](#cmd-website-publish)
- [Command: sync](#cmd-sync)
- [Command: token](#cmd-token)
- [Command: track](#cmd-track)
- [Command: track-blocks](#cmd-track-blocks)
- [Command: track-website](#cmd-track-website)
- [Command: flist](#cmd-flist)

<a id="cmd-mount"></a>
## Command: mount

- Purpose: mount an FL as a FUSE filesystem.
- Key options: meta, cache, daemon, log, target.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/fs/mod.rs](src/fs/mod.rs) ➜ [src/cache/mod.rs](src/cache/mod.rs) ➜ [src/store/mod.rs](src/store/mod.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcfsmodrs), [docs/function-index.md](docs/function-index.md#file-srccachemodrs), [docs/function-index.md](docs/function-index.md#file-srcstoremodrs)

<a id="cmd-pack"></a>
## Command: pack

- Purpose: create an FL and upload blocks to provided storage.
- Key options: meta, store, no_strip_password, target.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/pack.rs](src/pack.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcpackrs)

<a id="cmd-unpack"></a>
## Command: unpack

- Purpose: unpack (download) the content of an FL to a directory.
- Key options: meta, cache, preserve_ownership, target.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/unpack.rs](src/unpack.rs) ➜ [src/cache/mod.rs](src/cache/mod.rs) ➜ [src/store/mod.rs](src/store/mod.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcunpackrs), [docs/function-index.md](docs/function-index.md#file-srccachemodrs), [docs/function-index.md](docs/function-index.md#file-srcstoremodrs)

<a id="cmd-clone"></a>
## Command: clone

- Purpose: copy data from the stores of an FL to another stores configuration.
- Key options: meta, store, cache.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/clone.rs](src/clone.rs) ➜ [src/cache/mod.rs](src/cache/mod.rs) ➜ [src/store/mod.rs](src/store/mod.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-config"></a>
## Command: config

- Purpose: list or modify FL metadata and stores.
- Key options: meta; subcommands under tag and store.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/config.rs](src/config.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

### Subcommand: tag

- Purpose: manage tags in the FL metadata.
- Subcommands:
  - List — list existing tags.
  - Add — add key=value tags. Key options: tag.
  - Delete — delete by key or all. Key options: key, all.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/config.rs](src/config.rs)

### Subcommand: store

- Purpose: manage configured stores for the FL.
- Subcommands:
  - List — list stores.
  - Add — add stores. Key options: store.
  - Delete — delete by store or all. Key options: store, all.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/config.rs](src/config.rs)

<a id="cmd-merge"></a>
## Command: merge

- Purpose: merge two or more FLs into a new one.
- Key options: meta, server, token, target_flists, cache.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/merge.rs](src/merge.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcmergers)

<a id="cmd-docker"></a>
## Command: docker

- Purpose: convert a docker image to an FL.
- Key options: image_name, store, docker credentials (username, password, auth, email, server_address, identity_token, registry_token).
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/docker.rs](src/docker.rs) ➜ [src/upload.rs](src/upload.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcuploadrs)

<a id="cmd-server"></a>
## Command: server

- Purpose: run the server.
- Key options: config_path.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/server/mod.rs](src/server/mod.rs) ➜ [src/server/](src/server/)
- Notes: The CLI starts the server via app with a configuration file. HTTP handlers and models live under [src/server/](src/server/). Endpoint details are out of scope here; see [src/server/config.rs](src/server/config.rs) and directory contents for implementation.
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-upload"></a>
## Command: upload

- Purpose: upload a file to a server.
- Key options: path, server, block_size, token.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/upload.rs](src/upload.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcuploadrs)

<a id="cmd-upload-dir"></a>
## Command: upload-dir

- Purpose: upload a directory to a server; optionally create and output an flist.
- Key options: path, server, create_flist, flist_output, block_size, token.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/upload.rs](src/upload.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcuploadrs)

<a id="cmd-exists"></a>
## Command: exists

- Purpose: check a file or hash on a server; files are split into blocks for verification.
- Key options: file_or_hash, server, block_size.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/exist.rs](src/exist.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcexistrs)

<a id="cmd-download"></a>
## Command: download

- Purpose: download a file from a server using its hash.
- Key options: hash, output, server.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/download.rs](src/download.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-download-dir"></a>
## Command: download-dir

- Purpose: download a directory from a server using its flist hash.
- Key options: hash, output, server.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/download.rs](src/download.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-flist-create"></a>
## Command: flist-create

- Purpose: create an flist from a local directory and write it to an output file.
- Key options: directory, output, server, block_size, token.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/upload.rs](src/upload.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcuploadrs)

<a id="cmd-website-publish"></a>
## Command: website-publish

- Purpose: publish a static website to a server.
- Key options: path, server, block_size, token.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/upload.rs](src/upload.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcuploadrs)

<a id="cmd-sync"></a>
## Command: sync

- Purpose: sync files or blocks between two servers; optionally a specific file by hash.
- Key options: hash, source, destination, token.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/sync.rs](src/sync.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srcsyncrs)

<a id="cmd-token"></a>
## Command: token

- Purpose: retrieve an access token using username and password.
- Key options: username, password, server.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-track"></a>
## Command: track

- Purpose: track user blocks on the server.
- Key options: server, token, details.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-track-blocks"></a>
## Command: track-blocks

- Purpose: track block downloads; supports a specific block hash or all.
- Key options: server, token, hash or all, details.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-track-website"></a>
## Command: track-website

- Purpose: track downloads associated with a website flist hash.
- Key options: flist_hash, server, details.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/server_api.rs](src/server_api.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md)

<a id="cmd-flist"></a>
## Command: flist

- Purpose: flist inspection operations (tree, inspect).
- Key options: depend on subcommand (target; optional server_url).
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/tree_visitor.rs](src/tree_visitor.rs), [src/flist_inspector.rs](src/flist_inspector.rs)
- Related function-index: [docs/function-index.md](docs/function-index.md#file-srctree_visitorrs)

### Subcommand: tree

- Purpose: show tree structure of an flist (by path or by fetching via hash).
- Key options: target, server_url.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/download.rs](src/download.rs) ➜ [src/server_api.rs](src/server_api.rs) ➜ [src/tree_visitor.rs](src/tree_visitor.rs)

### Subcommand: inspect

- Purpose: inspect an flist by path or hash with a summary.
- Key options: target, server_url.
- Primary call path: [src/main.rs](src/main.rs) ➜ [src/download.rs](src/download.rs) ➜ [src/server_api.rs](src/server_api.rs) ➜ [src/flist_inspector.rs](src/flist_inspector.rs)