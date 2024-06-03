# FungiList specifications

## Introduction

The idea behind the FL format is to build a full filesystem description that is compact and also easy to use from almost ANY language. The format need to be easy to edit by tools like `rfs` or any other tool.

We decided to eventually use `sqlite`! Yes the `FL` file is just a `sqlite` database that has the following [schema](../schema/schema.sql)

## Tables

### Inode

Inode table describe each entry on the filesystem. It matches really closely the same `inode` structure on the linux operating system. Each inode has a unique id called `ino`, a parent `ino`, name, and other parameters (user, group, etc...).

The type of the `inode` is defined by its `mode` which is a `1:1` mapping from the linux `mode`

> from the [inode manual](https://man7.org/linux/man-pages/man7/inode.7.html)

```

POSIX refers to the stat.st_mode bits corresponding to the mask
S_IFMT (see below) as the file type, the 12 bits corresponding to
the mask 07777 as the file mode bits and the least significant 9
bits (0777) as the file permission bits.

The following mask values are defined for the file type:

    S_IFMT     0170000   bit mask for the file type bit field

    S_IFSOCK   0140000   socket
    S_IFLNK    0120000   symbolic link
    S_IFREG    0100000   regular file
    S_IFBLK    0060000   block device
    S_IFDIR    0040000   directory
    S_IFCHR    0020000   character device
    S_IFIFO    0010000   FIFO
```

## Extra

the `extra` table holds any **optional** data associated to the inode based on its type. For now it holds the `link target` for symlink inodes.

## Tag

tag is key value for some user defined data associated with the FL. The standard keys are:

- `version`
- `description`
- `author`

But an FL author can add other custom keys there

## Block

the `block` table is used to associate data file blocks with files. An `id` field is the blob `id` in the `store`, the `key` is the key used to decrypt the blob. The current implementation of `rfs` does the following:

- For each blob (512k) the `sha256`. This becomes the encryption key of the block. We call it `key`
- The block is then `snap` compressed
- Then encrypted with `aes_gcm` using the `key`, and the first 12 bytes of the key as `nonce`
- The final encrypted blocked is hashed again with `sha256` this becomes the `id` of the block
- The final encrypted blob is then sent to the store using the `id` as a key.

## Route

the route table holds routing information for the blobs. It basically describe where to find `blobs` with certain `ids`. The routing is done as following:

> Note routing table is loaded one time when `rfs` is started and

- We use the first byte of the blob `id` as the `route key`
- The `route key`` is then consulted against the routing table
- While building an `FL` all matching stores are updated with the new blob. This is how the system does replication
- On `getting` an object, the list of matching routes are tried in random order the first one to return a value is used
- Note that same range and overlapping ranges are allowed, this is how shards and replications are done.
