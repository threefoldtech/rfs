#![allow(clippy::unnecessary_mut_passed)]
#![deny(clippy::unimplemented, clippy::todo)]

use crate::cache;
use crate::fungi::{
    meta::{FileType, Inode},
    Reader,
};
use crate::store::Store;

use anyhow::{ensure, Result, Context};
use polyfuse::reply::FileAttr;
use polyfuse::{
    op,
    reply::{AttrOut, EntryOut, ReaddirOut, StatfsOut},
    KernelConfig, Operation, Request, Session,
};
use std::io::SeekFrom;
use std::sync::Arc;
use std::{io, path::PathBuf, time::Duration};
use tokio::fs::File;
use tokio::sync::Mutex;
use tokio::{
    io::{unix::AsyncFd, AsyncReadExt, AsyncSeekExt, Interest},
    task::{self, JoinHandle},
};

const CHUNK_SIZE: usize = 512 * 1024; // 512k and is hardcoded in the hub. the block_size value is not used
const TTL: Duration = Duration::from_secs(60 * 60 * 24 * 365);
const LRU_CAP: usize = 5; // Least Recently Used File Capacity
const FS_BLOCK_SIZE: u32 = 4 * 1024;

type FHash = [u8; 32];
type BlockSize = u64;

pub struct Filesystem<S>
where
    S: Store,
{
    meta: Reader,
    cache: Arc<cache::Cache<S>>,
    lru: Arc<Mutex<lru::LruCache<FHash, (File, BlockSize)>>>,
}

impl<S> Clone for Filesystem<S>
where
    S: Store,
{
    fn clone(&self) -> Self {
        Self {
            meta: self.meta.clone(),
            cache: Arc::clone(&self.cache),
            lru: Arc::clone(&self.lru),
        }
    }
}

impl<S> Filesystem<S>
where
    S: Store,
{
    pub fn new(meta: Reader, cache: cache::Cache<S>) -> Self {
        Filesystem {
            meta,
            cache: Arc::new(cache),
            lru: Arc::new(Mutex::new(lru::LruCache::new(LRU_CAP))),
        }
    }

    pub async fn mount<P>(&self, mnt: P) -> Result<()>
    where
        P: Into<PathBuf>,
    {
        let mountpoint: PathBuf = mnt.into();
        ensure!(mountpoint.is_dir(), "mountpoint must be a directory");
        let mut options = KernelConfig::default();
        options.mount_option(&format!(
            "ro,allow_other,fsname={},subtype=g8ufs,default_permissions",
            std::process::id()
        ));

        // polyfuse assumes an absolute path, see https://github.com/ubnt-intrepid/polyfuse/issues/83
        let fusermount_path =
            which::which("fusermount").context("looking up 'fusermount' in PATH")?;
        options.fusermount_path(fusermount_path);

        let session = AsyncSession::mount(mountpoint, options).await?;

        // release here
        while let Some(req) = session.next_request().await? {
            let fs = self.clone();

            let handler: JoinHandle<Result<()>> = task::spawn(async move {
                let result = match req.operation()? {
                    Operation::Lookup(op) => fs.lookup(&req, op).await,
                    Operation::Getattr(op) => fs.getattr(&req, op).await,
                    Operation::Read(op) => fs.read(&req, op).await,
                    Operation::Readdir(op) => fs.readdir(&req, op).await,
                    Operation::Readlink(op) => fs.readlink(&req, op).await,
                    Operation::Statfs(op) => fs.statfs(&req, op).await,
                    op => {
                        debug!("function is not implemented: {:?}", op);
                        Ok(req.reply_error(libc::ENOSYS)?)
                    }
                };

                if result.is_err() {
                    req.reply_error(libc::ENOENT)?;
                }

                Ok(())
            });

            drop(handler);
        }

        Ok(())
    }

    async fn statfs(&self, req: &Request, _op: op::Statfs<'_>) -> Result<()> {
        let mut out = StatfsOut::default();
        let stats = out.statfs();
        stats.bsize(FS_BLOCK_SIZE);
        req.reply(out)?;
        Ok(())
    }

    async fn readlink(&self, req: &Request, op: op::Readlink<'_>) -> Result<()> {
        let link = self.meta.inode(op.ino()).await?;
        if !link.mode.is(FileType::Link) {
            return Ok(req.reply_error(libc::ENOLINK)?);
        }

        if let Some(target) = link.data {
            req.reply(target)?;
            return Ok(());
        }

        Ok(req.reply_error(libc::ENOLINK)?)
    }

    async fn read(&self, req: &Request, op: op::Read<'_>) -> Result<()> {
        let entry = self.meta.inode(op.ino()).await?;

        if !entry.mode.is(FileType::Regular) {
            return Ok(req.reply_error(libc::EISDIR)?);
        };

        let offset = op.offset() as usize;
        let size = op.size() as usize;
        let chunk_size = CHUNK_SIZE; // file.block_size as usize;
        let chunk_index = offset / chunk_size;

        let blocks = self.meta.blocks(op.ino()).await?;

        if chunk_index >= blocks.len() || op.size() == 0 {
            // reading after the end of the file
            let data: &[u8] = &[];
            return Ok(req.reply(data)?);
        }

        // offset inside the file
        let mut offset = offset - (chunk_index * chunk_size);
        let mut buf: Vec<u8> = vec![0; size];
        let mut total = 0;

        'blocks: for block in blocks.iter().skip(chunk_index) {
            // hash works as a key inside the LRU
            let hash = block.id;

            // getting the file descriptor from the LRU or from the cache if not found in the LRU
            let lru = self.lru.lock().await.pop(&hash);

            let (mut fd, block_size) = match lru {
                Some((descriptor, bsize)) => {
                    debug!("lru hit");
                    (descriptor, bsize)
                }
                None => {
                    let (bsize, descriptor) = match self.cache.get(block).await {
                        Ok(out) => out,
                        Err(err) => {
                            error!("io cache error: {:#}", err);
                            return Ok(req.reply_error(libc::EIO)?);
                        }
                    };
                    (descriptor, bsize)
                }
            };

            // seek to the position <offset>
            fd.seek(SeekFrom::Start(offset as u64)).await?;

            let mut chunk_offset = offset as u64;

            loop {
                // read the file bytes into buf
                let read = match fd.read(&mut buf[total..]).await {
                    Ok(n) => n,
                    Err(err) => {
                        error!("read error: {:#}", err);
                        return Ok(req.reply_error(libc::EIO)?);
                    }
                };

                chunk_offset += read as u64;

                // calculate the total size and break if the required bytes (=size) downloaded
                total += read;

                if total >= size {
                    // if only part of the block read -> store it in the lruf
                    if chunk_offset < block_size {
                        let mut lruf = self.lru.lock().await;
                        lruf.put(hash, (fd, block_size));
                    }

                    break 'blocks;
                }

                // read = 0 means the EOF (end of the block)
                if read == 0 {
                    break;
                }
            }

            offset = 0;
        }

        Ok(req.reply(&buf[..size])?)
    }

    async fn getattr(&self, req: &Request, op: op::Getattr<'_>) -> Result<()> {
        log::debug!("getattr({})", op.ino());

        let entry = self.meta.inode(op.ino()).await?;

        let mut attr = AttrOut::default();

        let fill = attr.attr();
        entry.fill(fill);

        req.reply(attr)?;

        Ok(())
    }

    async fn readdir(&self, req: &Request, op: op::Readdir<'_>) -> Result<()> {
        log::debug!("readdir({})", op.ino());
        let root = self.meta.inode(op.ino()).await?;

        if !root.mode.is(FileType::Dir) {
            req.reply_error(libc::ENOTDIR)?;
            return Ok(());
        }

        let mut out = ReaddirOut::new(op.size() as usize);
        let mut offset = op.offset();

        let mut query_offset = offset;
        if offset == 0 {
            out.entry(".".as_ref(), op.ino(), libc::DT_DIR as u32, 1);
            out.entry(
                "..".as_ref(),
                match op.ino() {
                    1 => 1,
                    _ => root.parent,
                },
                libc::DT_DIR as u32,
                2,
            );
            offset = 2;
        } else {
            // we don't add the . and .. but
            // we also need to change the offset to
            query_offset -= 2;
        }

        let children = self.meta.children(root.ino, 10, query_offset).await?;
        for entry in children.iter() {
            offset += 1;

            let full = match entry.mode.file_type() {
                FileType::Dir => {
                    //let inode = self.meta.dir_inode(&sub.key).await?;
                    out.entry(entry.name.as_ref(), entry.ino, libc::DT_DIR as u32, offset)
                }
                FileType::Regular => {
                    out.entry(entry.name.as_ref(), entry.ino, libc::DT_REG as u32, offset)
                }
                FileType::Link => {
                    out.entry(entry.name.as_ref(), entry.ino, libc::DT_LNK as u32, offset)
                }
                _ => {
                    warn!("unkonwn entry");
                    false
                }
            };

            if full {
                break;
            }
        }

        Ok(req.reply(out)?)
    }

    async fn lookup(&self, req: &Request, op: op::Lookup<'_>) -> Result<()> {
        log::debug!("lookup(parent: {}, name: {:?})", op.parent(), op.name());
        let name = match op.name().to_str() {
            Some(name) => name,
            None => {
                req.reply_error(libc::ENOENT)?;
                return Ok(());
            }
        };

        let node = self.meta.lookup(op.parent(), name).await?;

        let node = match node {
            Some(node) => node,
            None => {
                req.reply_error(libc::ENOENT)?;
                return Ok(());
            }
        };
        let mut out = EntryOut::default();

        node.fill(out.attr());
        out.ino(node.ino);
        out.ttl_attr(TTL);
        out.ttl_entry(TTL);

        Ok(req.reply(out)?)
    }
}

// ==== AsyncSession ====

struct AsyncSession {
    inner: AsyncFd<Session>,
}

impl AsyncSession {
    async fn mount(mountpoint: PathBuf, config: KernelConfig) -> io::Result<Self> {
        tokio::task::spawn_blocking(move || {
            let session = Session::mount(mountpoint, config)?;
            Ok(Self {
                inner: AsyncFd::with_interest(session, Interest::READABLE)?,
            })
        })
        .await
        .expect("join error")
    }

    async fn next_request(&self) -> io::Result<Option<Request>> {
        use futures::{future::poll_fn, ready, task::Poll};

        poll_fn(|cx| {
            let mut guard = ready!(self.inner.poll_read_ready(cx))?;
            match self.inner.get_ref().next_request() {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    guard.clear_ready();
                    Poll::Pending
                }
                res => {
                    guard.retain_ready();
                    Poll::Ready(res)
                }
            }
        })
        .await
    }
}

trait AttributeFiller {
    fn fill(&self, attr: &mut FileAttr);
}

impl AttributeFiller for Inode {
    fn fill(&self, attr: &mut FileAttr) {
        attr.mode(self.mode.mode());

        attr.ino(self.ino);
        attr.ctime(Duration::from_secs(self.ctime as u64));
        attr.mtime(Duration::from_secs(self.mtime as u64));
        attr.uid(self.uid);
        attr.gid(self.gid);
        attr.size(self.size);
        attr.rdev(self.rdev as u32);
        attr.blksize(FS_BLOCK_SIZE);

        let mut blocks = self.size / 512;
        blocks += match self.size % 512 {
            0 => 0,
            _ => 1,
        };

        attr.blocks(blocks);

        match self.mode.file_type() {
            FileType::Dir => attr.nlink(2),
            FileType::Regular => attr.blksize(4 * 1024),
            _ => (),
        };
    }
}
