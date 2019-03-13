> This is a Work In Progress repository. It still under heavy development

# Introduction
This is (will be) a drop in replacement for [0-fs](https://github.com/threefoldtech/0-fs) written from scratch in rust.


## Motive
This is a `learning` project, trying to understand and get used to `rust`. A benchmarking is gonna be performed at the end of the implementation when we have all basic features, to compare the performance of both runtimes.


## Features
- [x] Mount an flist
- [x] Query file `stat`
- [x] Traverse the mount tree
- [x] Download files (chunks) from hub
- [x] Opening files for reading
- [x] Reading files
- [x] Command line interface
- [ ] ACL and ownership (now all dires and files have `0o755` and owned by `UID 0` (root) )
- [ ] Write support using `overlay` filesystem
- [ ] Download flist from flist (now you need to provide path to directory that has `flistdb.sqlite3`)

## Features that will never be implemented
- Runtime merging of flists (layering)

## Difference between the 2 implementations
- Rust fuse library only provide the Inode api (no higher lever API). This is the reason we can't merge flists in runtime, because `inodes` are
generated from `rawid` of the sqlite database.
- Rust fuse api does not support multi-threading.
- Downloaded files are stored in cache as chunks, so cache directory is not compatible between the 2 implementations.

## Limitations by the rust fuse framework
The rust fuse framework is not multi-threaded, BUT on opening a file that is not available in cache already we start multiple-threads to download the separate chunks in parallel.

## Basic usage
```
USAGE:
    rfs [FLAGS] [OPTIONS] <TARGET> --meta <META>

FLAGS:
        --debug      enable debug logging
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --cache <cache>        cache directory [default: /tmp/cache]
        --storage-url <hub>    storage url to retrieve files from [default: redis://hub.grid.tf:9900]
        --meta <META>          meta directory that has a .sqlite file from the flist

ARGS:
    <TARGET>    
```