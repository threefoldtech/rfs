
# Introduction
This is a drop in replacement for [0-fs](https://github.com/threefoldtech/0-fs) written from scratch in rust.

## Motive
Improve stability and resources needed to run a 0-fs instance.


## Features
- [x] Mount an flist
- [x] Query file `stat`
- [x] Traverse the mount tree
- [x] Download files (chunks) from hub
- [x] Opening files for reading
- [x] Reading files
- [x] Command line interface
- [x] ACL and ownership

## Difference between the 2 implementations
- Rust fuse library only provide the Inode api (no higher lever API). This is the reason we can't merge flists in runtime, because `inodes` are
generated from `rawid` of the sqlite database.

## Basic usage
```
Mount Flists 0.1

USAGE:
    rfs [FLAGS] [OPTIONS] <TARGET> --meta <META>

FLAGS:
    -d, --daemon     daemonize process
        --debug      enable debug logging
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --cache <cache>        cache directory [default: /tmp/cache]
        --storage-url <hub>    storage url to retrieve files from [default: redis://hub.grid.tf:9900]
        --log <log>            log file only in daemon mode
        --meta <META>          metadata file, can be a .flist file, a .sqlite3 file or a directory with a
                               `flistdb.sqlite3` inside

ARGS:
    <TARGET>
```
