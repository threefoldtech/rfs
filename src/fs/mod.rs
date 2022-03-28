#![allow(clippy::unnecessary_mut_passed)]
#![deny(clippy::unimplemented, clippy::todo)]

use crate::cache;
use crate::meta;

use anyhow::{ensure, Result};
use meta::types::EntryKind;
use polyfuse::{
    op,
    reply::{AttrOut, EntryOut, ReaddirOut, StatfsOut},
    KernelConfig, Operation, Request, Session,
};
use std::io::SeekFrom;
use std::sync::Arc;
use std::{io, os::unix::prelude::*, path::PathBuf, time::Duration};
use tokio::fs::File;
use tokio::sync::Mutex;
use tokio::{
    io::{unix::AsyncFd, AsyncReadExt, AsyncSeekExt, Interest},
    task::{self, JoinHandle},
};

const CHUNK_SIZE: usize = 512 * 1024; // 512k and is hardcoded in the hub. the block_size value is not used
const TTL: Duration = Duration::from_secs(60 * 60 * 24 * 365);
const LRU_CAP: usize = 5; // Least Recently Used File Capacity
type FHash = Vec<u8>;
type BlockSize = u64;

#[derive(Clone)]
pub struct Filesystem {
    meta: meta::Metadata,
    cache: cache::Cache,
    lru: Arc<Mutex<lru::LruCache<FHash, (File, BlockSize)>>>,
}

impl Filesystem {
    pub fn new(meta: meta::Metadata, cache: cache::Cache) -> Filesystem {
        Filesystem {
            meta,
            cache,
            lru: Arc::new(Mutex::new(lru::LruCache::new(LRU_CAP))),
        }
    }

    pub async fn mount<S>(&self, mnt: S) -> Result<()>
    where
        S: Into<PathBuf>,
    {
        let mountpoint: PathBuf = mnt.into();
        ensure!(mountpoint.is_dir(), "mountpoint must be a directory");
        let mut options = KernelConfig::default();
        options.mount_option(&format!(
            "ro,allow_other,fsname={},subtype=g8ufs,default_permissions",
            std::process::id()
        ));

        let session = AsyncSession::mount(mountpoint, options).await?;

        // release here
        while let Some(req) = session.next_request().await? {
            let fs = self.clone();

            let _: JoinHandle<Result<()>> = task::spawn(async move {
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
        }

        Ok(())
    }
    async fn statfs(&self, req: &Request, _op: op::Statfs<'_>) -> Result<()> {
        let out = StatfsOut::default();
        req.reply(out)?;
        Ok(())
    }

    async fn readlink(&self, req: &Request, op: op::Readlink<'_>) -> Result<()> {
        let entry = self.meta.entry(op.ino()).await?;
        let link = match entry.kind {
            EntryKind::Link(l) => l,
            _ => {
                return Ok(req.reply_error(libc::ENOLINK)?);
            }
        };

        req.reply(link.target)?;
        Ok(())
    }

    async fn read(&self, req: &Request, op: op::Read<'_>) -> Result<()> {
        let entry = self.meta.entry(op.ino()).await?;
        let file_metadata = match entry.kind {
            EntryKind::File(file) => file,
            _ => {
                return Ok(req.reply_error(libc::EISDIR)?);
            }
        };

        let offset = op.offset() as usize;
        let size = op.size() as usize;
        let chunk_size = CHUNK_SIZE; // file.block_size as usize;
        let chunk_index = offset / chunk_size;

        if chunk_index >= file_metadata.blocks.len() || op.size() == 0 {
            // reading after the end of the file
            let data: &[u8] = &[];
            return Ok(req.reply(data)?);
        }

        // offset inside the file
        let mut offset = offset - (chunk_index * chunk_size);
        let mut cache = self.cache.clone();
        let mut buf: Vec<u8> = vec![0; size];
        let mut total = 0;

        'blocks: for block in file_metadata.blocks.iter().skip(chunk_index) {
            // hash works as a key inside the LRU
            let hash = block.hash.clone();

            // getting the file descriptor from the LRU or from the cache if not found in the LRU
            let lru = self.lru.lock().await.pop(&hash);

            let (mut fd, block_size) = match lru {
                Some((descriptor, bsize)) => {
                    debug!("lru hit");
                    (descriptor, bsize)
                }
                None => {
                    let (bsize, descriptor) = match cache.get(block).await {
                        Ok(out) => out,
                        Err(_) => {
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
                    Err(_) => {
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
        let entry = self.meta.entry(op.ino()).await?;
        let mut attr = AttrOut::default();

        let fill = attr.attr();
        if entry.fill(&self.meta, fill).await.is_err() {
            req.reply_error(libc::ENOENT)?;
        }

        if op.ino() == 1 {
            fill.mode(libc::S_IFDIR | 0o755);
        }
        req.reply(attr)?;

        Ok(())
    }

    async fn readdir(&self, req: &Request, op: op::Readdir<'_>) -> Result<()> {
        let entry = self.meta.entry(op.ino()).await?;

        let dir = match entry.kind {
            EntryKind::Dir(dir) => dir,
            _ => {
                req.reply_error(libc::ENOTDIR)?;
                return Ok(());
            }
        };

        let mut out = ReaddirOut::new(op.size() as usize);
        let mut offset = op.offset();
        if offset == 0 {
            out.entry(".".as_ref(), op.ino(), libc::DT_DIR as u32, 1);
            out.entry(
                "..".as_ref(),
                match op.ino() {
                    1 => 1,
                    _ => self.meta.dir_inode(dir.parent).await?.ino(),
                },
                libc::DT_DIR as u32,
                2,
            );
        } else {
            // we don't add the . and .. but
            // we also need to change the offset to
            offset -= 2;
        }

        for (i, entry) in dir.entries.iter().enumerate().skip(offset as usize) {
            let offset = i as u64 + 3;
            let full = match &entry.kind {
                EntryKind::SubDir(sub) => {
                    let inode = self.meta.dir_inode(&sub.key).await?;
                    out.entry(
                        entry.node.name.as_ref(),
                        inode.ino(),
                        libc::DT_DIR as u32,
                        offset,
                    )
                }
                EntryKind::File(_) => out.entry(
                    entry.node.name.as_ref(),
                    entry.node.inode.ino(),
                    libc::DT_REG as u32,
                    offset,
                ),
                EntryKind::Link(_) => out.entry(
                    entry.node.name.as_ref(),
                    entry.node.inode.ino(),
                    libc::DT_LNK as u32,
                    offset,
                ),
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
        let parent = self.meta.entry(op.parent()).await?;

        let dir = match parent.kind {
            EntryKind::Dir(dir) => dir,
            _ => {
                req.reply_error(libc::ENOENT)?;
                return Ok(());
            }
        };

        for entry in dir.entries.iter() {
            if entry.node.name.as_bytes() == op.name().as_bytes() {
                let mut out = EntryOut::default();
                let inode = entry.fill(&self.meta, out.attr()).await?;
                out.ino(inode.ino());
                out.ttl_attr(TTL);
                out.ttl_entry(TTL);
                return Ok(req.reply(out)?);
            }
        }

        Ok(req.reply_error(libc::ENOENT)?)
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
