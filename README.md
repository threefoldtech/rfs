> This is a Work In Progress repository. It still under heavy development

# Introduction
This is (will be) a drop in replacement for [0-fs](https://github.com/threefoldtech/0-fs) written from scratch in rust.


## Motive
This is a `learning` project, trying to understand and get used to `rust`. A benchmarking is gonna be performed at the end of the implementation when we have all basic features, to compare the performance of both runtimes.


## Features
- [x] Mount an flist
- [x] Query file `stat`
- [x] Traverse the mount tree
- [ ] Download files (chunks) from hub
- [ ] Write support using `overlay` filesystem
- [ ] Command line interface

## Features that will never be implemented
- Runtime merging of flists (layering)

## Difference between the 2 implementations
- Rust fuse library only provide the Inode api (no higher lever API). This is the reason we can't merge flists in runtime, because `inodes` are
generated from `rawid` of the sqlite database.
- Rust fuse api does not support multi-threading.
- Downloaded files are stored in cache as chunks, so cache directory is not compatible between the 2 implementations.
