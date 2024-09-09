
# Introduction

`rfs` is the main tool to create, mount and extract FungiStore lists (FungiList)`fl` for short. An `fl` is a simple format
to keep information about an entire filesystem in a compact form. It does not hold the data itself but enough information to
retrieve this data back from a `store`.

## Building rfs

To build rfs make sure you have rust installed then run the following commands:

```bash
# this is needed to be run once to make sure the musl target is installed
rustup target add x86_64-unknown-linux-musl

# build the binary
cargo build --features build-binary --release --target=x86_64-unknown-linux-musl
```

the binary will be available under `./target/x86_64-unknown-linux-musl/release/rfs` you can copy that binary then to `/usr/bin/`
to be able to use from anywhere on your system.

## Stores

A store in where the actual data lives. A store can be as simple as a `directory` on your local machine in that case the files on the `fl` are only 'accessible' on your local machine. A store can also be a `zdb` running remotely or a cluster of `zdb`. Right now only `dir`, `zdb` and `s3` stores are supported but this will change in the future to support even more stores.

## Usage

### Creating an `fl`

```bash
rfs pack -m output.fl -s <store-specs> <directory>
```

This tells rfs to create an `fl` named `output.fl` using the store defined by the url `<store-specs>` and upload all the files under directory recursively.

The simplest form of `<store-specs>` is a `url`. the store `url` defines the store to use. Any `url`` has a schema that defines the store type. Right now we have support only for:

- `dir`: dir is a very simple store that is mostly used for testing. A dir store will store the fs blobs in another location defined by the url path. An example of a valid dir url is `dir:///tmp/store`
- `zdb`: [zdb](https://github.com/threefoldtech/0-db) is a append-only key value store and provides a redis like API. An example zdb url can be something like `zdb://<hostname>[:port][/namespace]`
- `s3`: aws-s3 is used for storing and retrieving large amounts of data (blobs) in buckets (directories). An example `s3://<username>:<password>@<host>:<port>/<bucket-name>`

  `region` is an optional param for s3 stores, if you want to provide one you can add it as a query to the url `?region=<region-name>`

`<store-specs>` can also be of the form `<start>-<end>=<url>` where `start` and `end` are a hex bytes for partitioning of blob keys. rfs will then store a set of blobs on the defined store if they blob key falls in the `[start:end]` range (inclusive).

If the `start-end` range is not provided a `00-FF` range is assume basically a catch all range for the blob keys. In other words, all blobs will be written to that store.

This is only useful because `rfs` can accept multiple stores on the command line with different and/or overlapping ranges.

For example `-s 00-80=dir:///tmp/store0 -s 81-ff=dir://tmp/store1` means all keys that has prefix byte in range `[00-80]` will be written to /tmp/store0 all other keys `[81-ff]` will be written to store1.

The same range can appear multiple times, which means the blob will be replicated to all the stores that matches its key prefix.

To quickly test this operation

```bash
rfs pack -m output.fl -s 00-80=dir:///tmp/store0 -s 81-ff=dir:///tmp/store1 ~/Documents
```

this command will effectively create the `output.fl` and store (and shard) the blobs across the 2 locations /tmp/store0 and /tmp/store1.

```bash
#rfs pack --help

create an FL and upload blocks to provided storage

Usage: rfs pack [OPTIONS] --meta <META> <TARGET>

Arguments:
  <TARGET>  target directory to upload

Options:
  -m, --meta <META>    path to metadata file (flist)
  -s, --store <STORE>  store url in the format [xx-xx=]<url>. the range xx-xx is optional and used for sharding. the URL is per store type, please check docs for more information
      --no-strip-password  disables automatic password stripping from store url, otherwise password will be stored in the fl.
  -h, --help           Print help
```

#### Password stripping

During creation of an flist you will probably provide a password in the URL of the store. This is normally needed to allow write operation to the store (say s3 bucket)
Normally this password is removed from the store info so it's safe to ship the fl to users. A user of the flist then will only have read access, if configured correctly
in the store

For example a `zdb` store has the notion of a public namespace which is password protected for writes, but open for reads. An S3 bucket can have the policy to allow public reads, but protected writes (minio supports that via bucket settings)

If you wanna disable the password stripping from the store url, you can provide the `--no-strip-password` flag during creation. This also means someone can extract
this information from the fl and gain write access to your store, so be careful how u use it.

# Mounting an `fl`

Once the `fl` is created it can be distributes to other people. Then they can mount the `fl` which will allow them then to traverse the packed filesystem and also access (read-only) the files.

To mount an `fl` only the `fl` is needed since all information regarding the `stores` is already stored in the `fl`. This also means you can only share the `fl` if the other user can actually reach the store used to crate the `fl`. So a `dir` store is not sharable, also a `zdb` instance that is running on localhost :no_good:

```bash
sudo rfs mount -m output.fl <target>
```

The `<target>` is the mount location, usually `/mnt` but can be anywhere. In another terminal you can now `cd <target>` and walk the filesystem tree. Opening the files will trigger a file download from the store only on read access.

full command help

```bash
# rfs mount --help

mount an FL

Usage: rfs mount [OPTIONS] --meta <META> <TARGET>

Arguments:
  <TARGET>  target mountpoint

Options:
  -m, --meta <META>    path to metadata file (flist)
  -c, --cache <CACHE>  directory used as cache for downloaded file chuncks [default: /tmp/cache]
  -d, --daemon         run in the background
  -l, --log <LOG>      log file only used with daemon mode
  -h, --help           Print help
```

# Unpack an `fl`

Similar to `mount` rfs provides an `unpack` subcommand that downloads the entire content (extract) of an `fl` to a provided directory.

```bash
rfs unpack --help
unpack (downloads) content of an FL the provided location

Usage: rfs unpack [OPTIONS] --meta <META> <TARGET>

Arguments:
  <TARGET>  target directory to upload

Options:
  -m, --meta <META>         path to metadata file (flist)
  -c, --cache <CACHE>       directory used as cache for downloaded file chuncks [default: /tmp/cache]
  -p, --preserve-ownership  preserve files ownership from the FL, otherwise use the current user ownership setting this flag to true normally requires sudo
  -h, --help                Print help
```

By default when unpacking the `-p` flag is not set. which means downloaded files will be `owned` by the current user/group. If `-p` flag is set, the files ownership will be same as the original files used to create the fl (preserve `uid` and `gid` of the files and directories) this normally requires `sudo` while unpacking.

# Specifications

Please check [docs](../docs)
