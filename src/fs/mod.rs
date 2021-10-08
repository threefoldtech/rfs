#![allow(clippy::unnecessary_mut_passed)]
#![deny(clippy::unimplemented, clippy::todo)]

use crate::cache;
use crate::meta;

use anyhow::{ensure, Result};
use meta::types::EntryKind;
use polyfuse::{
    op,
    reply::{AttrOut, EntryOut, ReaddirOut},
    KernelConfig, Operation, Request, Session,
};
use std::io::SeekFrom;
use std::{io, os::unix::prelude::*, path::PathBuf, time::Duration};
use tokio::{
    io::{unix::AsyncFd, AsyncReadExt, AsyncSeekExt, Interest},
    task::{self, JoinHandle},
};

const CHUNK_SIZE: usize = 512 * 1024; // 512k and is hardcoded in the hub. the block_size value is not used
const TTL: Duration = Duration::from_secs(60 * 60 * 24 * 365);

#[derive(Clone)]
pub struct Filesystem {
    meta: meta::Metadata,
    cache: cache::Cache,
}

impl Filesystem {
    pub fn new(meta: meta::Metadata, cache: cache::Cache) -> Filesystem {
        Filesystem { meta, cache }
    }

    pub async fn mount<S: Into<PathBuf>>(&self, mnt: S) -> Result<()> {
        let mountpoint: PathBuf = mnt.into();
        ensure!(mountpoint.is_dir(), "mountpoint must be a directory");
        let mut options = KernelConfig::default();
        options.mount_option(&format!(
            "ro,allow_other,fsname={},subtype=g8ufs",
            std::process::id()
        ));

        let session = AsyncSession::mount(mountpoint, options).await?;

        //let fs = Arc::new(Hello::new());

        while let Some(req) = session.next_request().await? {
            let fs = self.clone();

            let _: JoinHandle<Result<()>> = task::spawn(async move {
                let result = match req.operation()? {
                    Operation::Lookup(op) => fs.lookup(&req, op).await,
                    Operation::Getattr(op) => fs.getattr(&req, op).await,
                    Operation::Read(op) => fs.read(&req, op).await,
                    Operation::Readdir(op) => fs.readdir(&req, op).await,
                    Operation::Readlink(op) => fs.readlink(&req, op).await,
                    op => {
                        error!("got unknown operation: {:?}", op);
                        Ok(req.reply_error(libc::ENOSYS)?)
                    }
                };
                if let Err(_) = result {
                    req.reply_error(libc::ENOENT)?;
                }

                Ok(())
            });
        }

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
        let file = match entry.kind {
            EntryKind::File(file) => file,
            _ => {
                return Ok(req.reply_error(libc::EISDIR)?);
            }
        };

        let offset = op.offset() as usize;
        let size = op.size() as usize;
        let chunk_size = CHUNK_SIZE; // file.block_size as usize;
        let chunk_index = offset / chunk_size;

        if chunk_index >= file.blocks.len() || op.size() == 0 {
            // reading after the end of the file
            let data: &[u8] = &[];
            return Ok(req.reply(data)?);
        }

        // offset inside the file
        let mut offset = offset - (chunk_index * chunk_size);
        let mut cache = self.cache.clone();
        let mut buf: Vec<u8> = vec![0; size];
        let mut total = 0;

        'blocks: for block in file.blocks.iter().skip(chunk_index) {
            let (_, mut fd) = match cache.get(block).await {
                Ok(out) => out,
                Err(_) => {
                    return Ok(req.reply_error(libc::EIO)?);
                }
            };
            fd.seek(SeekFrom::Start(offset as u64)).await?;

            loop {
                let read = match fd.read(&mut buf[total..]).await {
                    Ok(n) => n,
                    Err(_) => {
                        return Ok(req.reply_error(libc::EIO)?);
                    }
                };
                total += read;
                if total >= size {
                    break 'blocks;
                }

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

        if let Err(_) = entry.fill(&self.meta, attr.attr()).await {
            req.reply_error(libc::ENOENT)?;
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
            offset = offset - 2;
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
