# docker2fl

`docker2fl` is a tool to extract docker images and convert them to flist using [rfs](../rfs) tool.

## Build

To build docker2fl make sure you have rust installed then run the following commands:

```bash
# this is needed to be run once to make sure the musl target is installed
rustup target add x86_64-unknown-linux-musl

# build the binary
cargo build --release --target=x86_64-unknown-linux-musl
```

the binary will be available under `./target/x86_64-unknown-linux-musl/release/docker2fl` you can copy that binary then to `/usr/bin/`
to be able to use from anywhere on your system.

```bash
sudo mv ./target/x86_64-unknown-linux-musl/release/docker2fl /usr/bin/
```

## Stores

A store in where the actual data lives. A store can be as simple as a `directory` on your local machine in that case the files on the `fl` are only 'accessible' on your local machine. A store can also be a `zdb` running remotely or a cluster of `zdb`. Right now only `dir`, `zdb` and `s3` stores are supported but this will change in the future to support even more stores.

## Usage

### Creating an `fl`

```bash
docker2fl -i redis -s <store-specs>
```

This tells docker2fl to create an `fl` named `redis-latest.fl` using the store defined by the url `<store-specs>` and upload all the files under the temp docker directory that include exported docker image recursively.

The simplest form of `<store-specs>` is a `url`. the store `url` defines the store to use. Any `url` has a schema that defines the store type. Right now we have support only for:

- `dir`: dir is a very simple store that is mostly used for testing. A dir store will store the fs blobs in another location defined by the url path. An example of a valid dir url is `dir:///tmp/store`
- `zdb`: [zdb](https://github.com/threefoldtech/0-db) is a append-only key value store and provides a redis like API. An example zdb url can be something like `zdb://<hostname>[:port][/namespace]`
- `s3`: aws-s3 is used for storing and retrieving large amounts of data (blobs) in buckets (directories). An example `s3://<username>:<password>@<host>:<port>/<bucket-name>`
  
  `region` is an optional param for s3 stores, if you want to provide one you can add it as a query to the url `?region=<region-name>`

`<store-specs>` can also be of the form `<start>-<end>=<url>` where `start` and `end` are a hex bytes for partitioning of blob keys. rfs will then store a set of blobs on the defined store if they blob key falls in the `[start:end]` range (inclusive).

If the `start-end` range is not provided a `00-FF` range is assume basically a catch all range for the blob keys. In other words, all blobs will be written to that store.

This is only useful because `docker2fl` can accept multiple stores on the command line with different and/or overlapping ranges.

For example `-s 00-80=dir:///tmp/store0 -s 81-ff=dir://tmp/store1` means all keys that has prefix byte in range `[00-80]` will be written to /tmp/store0 all other keys `00-ff` will be written to store1.

The same range can appear multiple times, which means the blob will be replicated to all the stores that matches its key prefix.

To quickly test this operation

```bash
docker2fl -i redis -s "dir:///tmp/store0"
```

this command will use redis image and effectively create the `redis.fl` and store (and shard) the blobs across the location /tmp/store0.

```bash
#docker2fl --help

Usage: docker2fl [OPTIONS] --image-name <IMAGE_NAME>
Options:
      --debug...
          enable debugging logs
  -i, --image-name <IMAGE_NAME>
          name of the docker image to be converted to flist
  -s, --store <STORE>
          store url for rfs in the format [xx-xx=]<url>. the range xx-xx is optional and used for sharding. the URL is per store type, please check docs for more information
  -h, --help
          Print help
  -V, --version
          Print version
```

## Generate an flist using ZDB

### Deploy a vm

1. Deploy a vm with a public IP
2. add docker (don't forget to add a disk for it with mountpoint = "/var/lib/docker")
3. add caddy

### Install zdb and run an instance of it

1. Execute `git clone -b development-v2 https://github.com/threefoldtech/0-db /zdb` then `cd /zdb`
2. Build

      ```bash
      cd libzdb
      make
      cd ..

      cd zdbd
      make STATIC=1
      cd ..

      make
      ```

3. Install `make install`
4. run `zdb --listen 0.0.0.0`
5. The result info you should know

      ```console
      zdbEndpoint = "<vm public IP>:<port>"
      zdbNameSpace = "default"
      zdbPassword = "default"
      ```

### Install docker2fl

1. Execute `git clone -b development-v2 https://github.com/threefoldtech/rfs` then `cd /rfs`
2. Execute

      ```bash
      rustup target add x86_64-unknown-linux-musl`
      cargo build --features build-binary --release --target=x86_64-unknown-linux-musl
      mv ./target/x86_64-unknown-linux-musl/release/docker2fl /usr/bin/
      ```

### Convert docker image to an fl

1. Try an image for example `threefolddev/ubuntu:22.04` image
2. Executing `docker2fl -i threefolddev/ubuntu:22.04 -s "zdb://<vm public IP>:<port>/default" -d`
3. You will end up having `threefolddev-ubuntu-22.04.fl` (flist)

### Serve the flist using caddy

1. In the directory includes the output flist, you can run `caddy file-server --listen 0.0.0.0:2015 --browse`
2. The flist will be available as `http://<vm public IP>:2015/threefolddev-ubuntu-22.04.fl`
3. Use the flist to deploy any virtual machine.
