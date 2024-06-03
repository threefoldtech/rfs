# rfs

This repo contains the binaries related to rfs.

[![Test](https://github.com/threefoldtech/rfs/actions/workflows/tests.yaml/badge.svg?branch=master)](https://github.com/threefoldtech/rfs/actions/workflows/tests.yaml)

## Introduction

`rfs` is the main tool to create, mount and extract FungiStore lists (FungiList)`fl` for short. An `fl` is a simple format
to keep information about an entire filesystem in a compact form. It does not hold the data itself but enough information to
retrieve this data back from a `store`.

## Build

Make sure you have rust installed then run the following commands:

```bash
# this is needed to be run once to make sure the musl target is installed
rustup target add x86_64-unknown-linux-musl

# build all binaries
cargo build --features build-binary --release --target=x86_64-unknown-linux-musl
```

The rfs binary will be available under `./target/x86_64-unknown-linux-musl/release/rfs`

The docker2fl binary will be available under `./target/x86_64-unknown-linux-musl/release/docker2fl`

you can copy the binaries then to `/usr/bin/` to be able to use from anywhere on your system.

## Binaries and libraries

-   [rfs](./rfs/README.md)
-   [docker2fl](./docker2fl/README.md)
