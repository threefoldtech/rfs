use crate::fungi::meta::{Ino, Inode};
use crate::fungi::{Result, Writer};
use crate::store::{BlockStore, Store};
use anyhow::Context;
use std::collections::LinkedList;
use std::ffi::OsString;
use std::fs::Metadata;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use workers::WorkerPool;

const BLOB_SIZE: usize = 512 * 1024; // 512K

#[derive(Debug)]
struct Item(Ino, PathBuf, OsString, Metadata);
/// creates an FL from the given root location. It takes ownership of the writer because
/// it's logically incorrect to store multiple filessytem in the same FL.
/// All file chunks will then be uploaded to the provided store
///
pub async fn pack<P: Into<PathBuf>, S: Store>(
    writer: Writer,
    store: S,
    root: P,
    strip_password: bool,
) -> Result<()> {
    use tokio::fs;

    // building routing table from store information
    for route in store.routes() {
        let mut store_url = route.url;

        if strip_password {
            let mut url = url::Url::parse(&store_url).context("failed to parse store url")?;
            if url.password().is_some() {
                url.set_password(None)
                    .map_err(|_| anyhow::anyhow!("failed to strip password"))?;

                store_url = url.to_string();
            }
        }

        writer
            .route(
                route.start.unwrap_or(u8::MIN),
                route.end.unwrap_or(u8::MAX),
                store_url,
            )
            .await?;
    }

    let store: BlockStore<S> = store.into();

    let root = root.into();
    let meta = fs::metadata(&root)
        .await
        .context("failed to get root stats")?;

    let mut list = LinkedList::default();

    let uploader = Uploader::new(store, writer.clone());
    let mut pool = workers::WorkerPool::new(uploader.clone(), super::PARALLEL_UPLOAD);

    pack_one(
        &mut list,
        &writer,
        &mut pool,
        Item(0, root, OsString::from("/"), meta),
    )
    .await?;

    while !list.is_empty() {
        let dir = list.pop_back().unwrap();
        pack_one(&mut list, &writer, &mut pool, dir).await?;
    }

    pool.close().await;
    Ok(())
}

/// pack_one is called for each dir
async fn pack_one<S: Store>(
    list: &mut LinkedList<Item>,
    writer: &Writer,
    pool: &mut WorkerPool<Uploader<S>>,
    Item(parent, path, name, meta): Item,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    use tokio::fs;

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

        let worker = pool.get().await;
        worker
            .send((child_ino, child_path))
            .context("failed to schedule file upload")?;
    }
    Ok(())
}

struct Uploader<S>
where
    S: Store,
{
    store: Arc<BlockStore<S>>,
    writer: Writer,
    buffer: [u8; BLOB_SIZE],
}

impl<S> Clone for Uploader<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            writer: self.writer.clone(),
            buffer: [0; BLOB_SIZE],
        }
    }
}

impl<S> Uploader<S>
where
    S: Store,
{
    fn new(store: BlockStore<S>, writer: Writer) -> Self {
        Self {
            store: Arc::new(store),
            writer,
            buffer: [0; BLOB_SIZE],
        }
    }

    async fn upload(&mut self, ino: Ino, path: &Path) -> Result<()> {
        use tokio::fs;
        use tokio::io::AsyncReadExt;
        use tokio::io::BufReader;

        // create file blocks
        let fd = fs::OpenOptions::default().read(true).open(path).await?;

        let mut reader = BufReader::new(fd);
        loop {
            let size = reader.read(&mut self.buffer).await?;
            if size == 0 {
                break;
            }

            // write block to remote store
            let block = self
                .store
                .set(&self.buffer[..size])
                .await
                .context("failed to store blob")?;

            // write block info to meta
            self.writer.block(ino, &block.id, &block.key).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<S> workers::Work for Uploader<S>
where
    S: Store,
{
    type Input = (Ino, PathBuf);
    type Output = ();

    async fn run(&mut self, (ino, path): Self::Input) -> Self::Output {
        if let Err(err) = self.upload(ino, &path).await {
            log::error!("failed to upload file ({:?}): {:#}", path, err);
        }
    }
}
