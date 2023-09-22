#[macro_use]
extern crate log;
use anyhow::Context;
use fungi::meta::Ino;
use fungi::Writer;
use nix::unistd::{fchownat, FchownatFlags, Gid, Uid};
use std::collections::LinkedList;
use std::ffi::OsString;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{ffi::OsStr, fs};
use store::Store;

const BLOB_SIZE: usize = 512 * 1024; // 512K

pub mod cache;
pub mod fungi;
pub mod store;

use cache::Cache;
use fungi::{
    meta::{FileType, Inode, Result, Walk, WalkVisitor},
    Reader,
};

use crate::store::BlockStore;

struct CopyVisitor<'a, S>
where
    S: store::Store,
{
    preserve: bool,
    meta: &'a fungi::Reader,
    cache: &'a cache::Cache<S>,
    root: &'a Path,
}

impl<'a, S> CopyVisitor<'a, S>
where
    S: store::Store,
{
    pub fn new(
        meta: &'a fungi::Reader,
        cache: &'a Cache<S>,
        root: &'a Path,
        preserve: bool,
    ) -> Self {
        Self {
            meta,
            cache,
            root,
            preserve,
        }
    }
}

#[async_trait::async_trait]
impl<'a, S> WalkVisitor for CopyVisitor<'a, S>
where
    S: Store,
{
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        use tokio::fs::OpenOptions;

        let rooted = self.root.join(path.strip_prefix("/").unwrap());

        match node.mode.file_type() {
            FileType::Dir => {
                fs::create_dir_all(&rooted)
                    .with_context(|| format!("failed to create directory '{:?}'", rooted))?;
            }
            FileType::Regular => {
                let mut fd = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .truncate(true)
                    .open(&rooted)
                    .await
                    .with_context(|| format!("failed to create file '{:?}'", rooted))?;

                let blocks = self.meta.blocks(node.ino).await?;
                self.cache
                    .direct(&blocks, &mut fd)
                    .await
                    .with_context(|| format!("failed to download file '{:?}'", rooted))?;

                fd.set_permissions(Permissions::from_mode(node.mode.mode()))
                    .await?;
            }
            FileType::Link => {
                let target = node
                    .data
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("link has no target path"))?;

                let target = Path::new(OsStr::from_bytes(target));
                let target = if target.is_relative() {
                    target.to_owned()
                } else {
                    self.root.join(target)
                };

                std::os::unix::fs::symlink(target, &rooted)
                    .with_context(|| format!("failed to create symlink '{:?}'", rooted))?;
            }
            _ => {
                warn!("unknown file kind: {:?}", node.mode.file_type());
                return Ok(Walk::Continue);
            }
        };

        if self.preserve {
            fchownat(
                None,
                &rooted,
                Some(Uid::from_raw(node.uid)),
                Some(Gid::from_raw(node.gid)),
                FchownatFlags::NoFollowSymlink,
            )
            .with_context(|| format!("failed to change ownership of '{:?}'", &rooted))?;
        }

        Ok(Walk::Continue)
    }
}

/// unpack an FL to the given root location. it will download the files and reconstruct
/// the filesystem.
pub async fn unpack<P: AsRef<Path>, S: Store>(
    meta: &Reader,
    cache: &Cache<S>,
    root: P,
    preserve: bool,
) -> Result<()> {
    let mut visitor = CopyVisitor::new(meta, cache, root.as_ref(), preserve);

    meta.walk(&mut visitor).await
}

#[derive(Debug)]
struct Item(Ino, PathBuf, OsString, Metadata);
/// creates an FL from the given root location. It takes ownership of the writer because
/// it's logically incorrect to store multiple filessytem in the same FL.
/// All file chunks will then be uploaded to the provided store
///
pub async fn pack<P: Into<PathBuf>, S: Store>(writer: Writer, store: S, root: P) -> Result<()> {
    use tokio::fs;

    // building routing table from store information
    for route in store.routes() {
        writer
            .route(
                route.start.unwrap_or(u8::MIN),
                route.end.unwrap_or(u8::MAX),
                route.url,
            )
            .await?;
    }

    let store: BlockStore<S> = store.into();

    let root = root.into();
    let meta = fs::metadata(&root)
        .await
        .context("failed to get root stats")?;

    let mut list = LinkedList::default();

    pack_one(
        &mut list,
        &writer,
        &store,
        Item(0, root, OsString::from("/"), meta),
    )
    .await?;

    while !list.is_empty() {
        let dir = list.pop_back().unwrap();

        pack_one(&mut list, &writer, &store, dir).await?;
    }

    Ok(())
}

/// pack_one is called for each dir
async fn pack_one<S: Store>(
    list: &mut LinkedList<Item>,
    writer: &Writer,
    store: &BlockStore<S>,
    Item(parent, path, name, meta): Item,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    use tokio::fs;
    use tokio::io::AsyncReadExt;
    use tokio::io::BufReader;

    let current = writer
        .inode(Inode {
            ino: 0,
            name: String::from_utf8_lossy(name.as_bytes()).into_owned(),
            parent,
            size: meta.size(),
            uid: meta.uid(),
            gid: meta.gid(),
            mode: meta.mode().into(),
            rdev: meta.rdev(),
            ctime: meta.ctime(),
            mtime: meta.mtime(),
            data: None,
        })
        .await?;

    let mut children = fs::read_dir(&path)
        .await
        .context("failed to list dir children")?;

    while let Some(child) = children
        .next_entry()
        .await
        .context("failed to read next entry from directory")?
    {
        let name = child.file_name();
        let meta = child.metadata().await?;
        let child_path = path.join(&name);

        // if this child a directory we add to the tail of the list
        if meta.is_dir() {
            list.push_back(Item(current, child_path.clone(), name, meta));
            continue;
        }

        // create entry
        // otherwise create the file meta
        let data = if meta.is_symlink() {
            let target = fs::read_link(&child_path).await?;
            Some(target.as_os_str().as_bytes().into())
        } else {
            None
        };

        let child_ino = writer
            .inode(Inode {
                ino: 0,
                name: String::from_utf8_lossy(name.as_bytes()).into_owned(),
                parent: current,
                size: meta.size(),
                uid: meta.uid(),
                gid: meta.gid(),
                mode: meta.mode().into(),
                rdev: meta.rdev(),
                ctime: meta.ctime(),
                mtime: meta.mtime(),
                data,
            })
            .await?;

        if !meta.is_file() {
            continue;
        }

        // create file blocks
        let fd = fs::OpenOptions::default()
            .read(true)
            .open(&child_path)
            .await?;
        let mut reader = BufReader::new(fd);
        let mut buffer: [u8; BLOB_SIZE] = [0; BLOB_SIZE];
        loop {
            let size = reader.read(&mut buffer).await?;
            if size == 0 {
                break;
            }

            // write block to remote store
            let block = store
                .set(&buffer[..size])
                .await
                .context("failed to store blob")?;

            // write block info to meta
            writer.block(child_ino, &block.id, &block.key).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        fungi::meta,
        store::{dir::DirStore, Router},
    };
    use std::path::PathBuf;
    use tokio::{fs, io::AsyncReadExt};

    #[tokio::test]
    async fn pack_unpack() {
        const ROOT: &str = "/tmp/pack-unpack-test";
        let _ = fs::remove_dir_all(ROOT).await;

        let root: PathBuf = ROOT.into();
        let source = root.join("source");
        fs::create_dir_all(&source).await.unwrap();

        for size in [0, 100 * 1024, 1024 * 1024, 10 * 1024 * 1024] {
            let mut urandom = fs::OpenOptions::default()
                .read(true)
                .open("/dev/urandom")
                .await
                .unwrap()
                .take(size);

            let name = format!("file-{}.rnd", size);
            let p = source.join(&name);
            let mut file = fs::OpenOptions::default()
                .create(true)
                .write(true)
                .open(p)
                .await
                .unwrap();

            tokio::io::copy(&mut urandom, &mut file).await.unwrap();
        }

        println!("file generation complete");
        let writer = meta::Writer::new(root.join("meta.fl")).await.unwrap();

        // while we at it we can already create 2 stores and create a router store on top
        // of that.
        let store0 = DirStore::new(root.join("store0")).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut store = Router::new();

        store.add(0x00, 0x7f, Box::new(store0));
        store.add(0x80, 0xff, Box::new(store1));

        pack(writer, store, &source).await.unwrap();

        println!("packing complete");
        // recreate the stores for reading.
        let store0 = DirStore::new(root.join("store0")).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut store = Router::new();

        store.add(0x00, 0x7f, Box::new(store0));
        store.add(0x80, 0xff, Box::new(store1));

        let cache = Cache::new(root.join("cache"), store);

        let reader = meta::Reader::new(root.join("meta.fl")).await.unwrap();
        // validate reader store routing
        let routers = reader.routes().await.unwrap();
        assert_eq!(2, routers.len());
        assert_eq!(routers[0].url, "dir:///tmp/pack-unpack-test/store0");
        assert_eq!(routers[1].url, "dir:///tmp/pack-unpack-test/store1");

        assert_eq!((routers[0].start, routers[0].end), (0x00, 0x7f));
        assert_eq!((routers[1].start, routers[1].end), (0x80, 0xff));

        unpack(&reader, &cache, root.join("destination"), false)
            .await
            .unwrap();

        println!("unpacking complete");
        // compare that source directory is exactly the same as target directory
        let status = std::process::Command::new("diff")
            .arg(root.join("source"))
            .arg(root.join("destination"))
            .status()
            .unwrap();

        assert!(status.success());
    }
}
