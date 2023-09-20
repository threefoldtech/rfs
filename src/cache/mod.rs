use crate::fungi::meta::Block;
use crate::store::{BlockStore, Store};
use anyhow::{Context, Result};

use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

/// Cache implements a caching layer on top of a block store
#[derive(Clone)]
pub struct Cache<S: Store> {
    store: BlockStore<S>,
    root: PathBuf,
}

impl<S> Cache<S>
where
    S: Store,
{
    pub fn new<P>(root: P, store: S) -> Self
    where
        P: Into<PathBuf>,
    {
        Cache {
            store: store.into(),
            root: root.into(),
        }
    }

    // download given an open file, writes the content of the chunk to the file
    async fn download(&self, file: &mut File, block: &Block) -> Result<u64> {
        let data = self.store.get(block).await?;
        file.write_all(&data).await?;

        Ok(data.len() as u64)
    }

    async fn prepare(&self, id: &[u8]) -> Result<File> {
        let name = id.hex();
        if name.len() < 4 {
            anyhow::bail!("invalid chunk hash");
        }
        let path = self.root.join(&name[0..2]).join(&name[2..4]);
        fs::create_dir_all(&path).await?;
        let path = path.join(name);

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)
            .await?;

        Ok(file)
    }

    /// get a file block either from cache or from remote if it's already
    /// not cached
    pub async fn get(&self, block: &Block) -> Result<(u64, File)> {
        let mut file = self.prepare(&block.id).await?;
        // TODO: locking must happen here so no
        // other processes start downloading the same chunk
        let locker = Locker::new(&file);
        locker.lock().await?;

        let meta = file.metadata().await?;
        if meta.len() > 0 {
            // chunk is already downloaded
            debug!("block cache hit: {}", block.id.as_slice().hex());
            locker.unlock().await?;
            return Ok((meta.len(), file));
        }

        debug!("downloading block: {}", block.id.as_slice().hex());
        let size = self.download(&mut file, block).await?;

        // if file is just downloaded, we need
        // to seek to beginning of the file.
        file.rewind().await?;

        locker.unlock().await?;
        Ok((size, file))
    }

    /// direct downloads all the file blocks from remote and write it to output
    #[allow(dead_code)]
    pub async fn direct(&self, blocks: &[Block], out: &mut File) -> Result<()> {
        use tokio::io::copy;
        for (index, block) in blocks.iter().enumerate() {
            let (_, mut chunk) = self.get(block).await?;
            copy(&mut chunk, out)
                .await
                .with_context(|| format!("failed to download block {}", index))?;
        }

        Ok(())
    }
}

pub struct Locker {
    fd: std::os::unix::io::RawFd,
}

impl Locker {
    pub fn new(f: &File) -> Locker {
        Locker { fd: f.as_raw_fd() }
    }

    pub async fn lock(&self) -> Result<()> {
        let fd = self.fd;
        tokio::task::spawn_blocking(move || {
            nix::fcntl::flock(fd, nix::fcntl::FlockArg::LockExclusive)
        })
        .await
        .context("failed to spawn file locking")?
        .context("failed to lock file")?;

        Ok(())
    }

    pub async fn unlock(&self) -> Result<()> {
        let fd = self.fd;
        tokio::task::spawn_blocking(move || nix::fcntl::flock(fd, nix::fcntl::FlockArg::Unlock))
            .await
            .context("failed to spawn file lunlocking")?
            .context("failed to unlock file")?;

        Ok(())
    }
}

trait Hex {
    fn hex(&self) -> String;
}

impl Hex for &[u8] {
    fn hex(&self) -> String {
        hex::encode(self)
    }
}
