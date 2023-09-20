#[macro_use]
extern crate log;
use anyhow::Context;
use fungi::meta::Ino;
use fungi::Writer;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
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
    meta: &'a fungi::Reader,
    cache: &'a cache::Cache<S>,
    root: &'a Path,
}

impl<'a, S> CopyVisitor<'a, S>
where
    S: store::Store,
{
    pub fn new(meta: &'a fungi::Reader, cache: &'a Cache<S>, root: &'a Path) -> Self {
        Self { meta, cache, root }
    }
}

#[async_trait::async_trait]
impl<'a, S> WalkVisitor for CopyVisitor<'a, S>
where
    S: Store,
{
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
        use tokio::fs::OpenOptions;

        let rooted = self.root.join(path.strip_prefix("/").unwrap());

        match node.mode.file_type() {
            FileType::Dir => {
                fs::create_dir_all(&rooted)
                    .with_context(|| format!("failed to create directory '{:?}'", rooted))?;
            }
            FileType::Regular => {
                let mut fd = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .mode(node.mode.mode())
                    .open(&rooted)
                    .await
                    .with_context(|| format!("failed to create file '{:?}'", rooted))?;

                let blocks = self.meta.blocks(node.ino).await?;
                self.cache
                    .direct(&blocks, &mut fd)
                    .await
                    .with_context(|| format!("failed to create download file '{:?}'", rooted))?;
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
                debug!("unknown file kind: {:?}", node.mode.file_type());
            }
        };

        Ok(Walk::Continue)
    }
}

/// unpack an FL to the given root location. it will download the files and reconstruct
/// the filesystem.
pub async fn unpack<P: AsRef<Path>, S: Store>(
    meta: &Reader,
    cache: &Cache<S>,
    root: P,
) -> Result<()> {
    let mut visitor = CopyVisitor::new(meta, cache, root.as_ref());

    meta.walk(&mut visitor).await
}

/// creates an FL from the given root location. It takes ownership of the writer because
/// it's logically incorrect to store multiple filessytem in the same FL.
/// All file chunks will then be uploaded to the provided store
///
pub async fn pack<P: AsRef<Path>, S: Store>(meta: Writer, store: S, root: P) -> Result<()> {
    use tokio::fs;

    // building routing table from store information
    for route in store.routes() {
        meta.route(
            route.start.unwrap_or(u8::MIN),
            route.end.unwrap_or(u8::MAX),
            route.url,
        )
        .await?;
    }

    let store: BlockStore<S> = store.into();

    let m = fs::metadata(&root)
        .await
        .context("failed to get root stats")?;

    scan(&meta, &store, 0, "/", root.as_ref(), &m).await
}

#[async_recursion::async_recursion]
async fn scan<S: Store>(
    meta: &Writer,
    store: &BlockStore<S>,
    parent: Ino,
    name: &str,
    path: &Path,
    m: &Metadata,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    use tokio::fs;
    use tokio::io::AsyncReadExt;
    use tokio::io::BufReader;

    let data = if m.is_symlink() {
        let target = fs::read_link(&path).await?;
        Some(target.as_os_str().as_bytes().into())
    } else {
        None
    };

    let ino = meta
        .inode(Inode {
            name: name.into(),
            parent,
            size: m.size(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode().into(),
            rdev: m.rdev(),
            ctime: m.ctime(),
            mtime: m.mtime(),
            data,
            ..Default::default()
        })
        .await?;

    if m.is_file() {
        // create file blocks
        let fd = fs::OpenOptions::default().read(true).open(&path).await?;
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
            meta.block(ino, &block.id, &block.key).await?;
        }
    }

    if !m.is_dir() {
        return Ok(());
    }

    // sub files
    let mut children = fs::read_dir(path).await?;
    while let Some(child) = children.next_entry().await? {
        scan(
            meta,
            store,
            ino,
            &child.file_name().to_string_lossy(),
            &path.join(child.file_name()),
            &child.metadata().await?,
        )
        .await?;
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
    use rand::Rng;
    use std::path::PathBuf;
    use tokio::fs;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn pack_unpack() {
        const ROOT: &str = "/tmp/pack-unpack-test";
        fs::remove_dir_all(ROOT).await.unwrap();

        let root: PathBuf = ROOT.into();
        let source = root.join("source");
        fs::create_dir_all(&source).await.unwrap();

        let mut buffer: [u8; 1024] = [0; 1024];
        let mut rng = rand::thread_rng();
        // generate random files.
        for size in [0, 100 * 1024, 1024 * 1024, 10 * 1024 * 1024] {
            let name = format!("file-{}.rnd", size);
            let p = source.join(&name);
            let mut file = fs::OpenOptions::default()
                .create(true)
                .write(true)
                .open(p)
                .await
                .unwrap();

            let mut filled = 0;
            // fill it with random data
            loop {
                rng.fill(&mut buffer);
                file.write_all(&buffer).await.unwrap();
                filled += buffer.len();
                if filled >= size {
                    break;
                }
            }
        }

        let writer = meta::Writer::new(root.join("meta.fl")).await.unwrap();

        // while we at it we can already create 2 stores and create a router store on top
        // of that.
        let store0 = DirStore::new(root.join("store0")).await.unwrap();
        let store1 = DirStore::new(root.join("store1")).await.unwrap();
        let mut store = Router::new();

        store.add(0x00, 0x7f, Box::new(store0));
        store.add(0x80, 0xff, Box::new(store1));

        pack(writer, store, &source).await.unwrap();

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

        unpack(&reader, &cache, root.join("destination"))
            .await
            .unwrap();

        // compare that source directory is exactly the same as target directory
        let status = std::process::Command::new("diff")
            .arg(root.join("source"))
            .arg(root.join("destination"))
            .status()
            .unwrap();

        assert!(status.success());
    }

    struct WalkTest;

    #[async_trait::async_trait]
    impl WalkVisitor for WalkTest {
        async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
            println!("{} = {:?}", node.ino, path);
            Ok(Walk::Continue)
        }
    }
}
