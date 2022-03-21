use crate::meta::types::FileBlock;
use anyhow::{Context, Result};
//use fs2::FileExt;
use redis::Client;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

trait Hex {
    fn hex(&self) -> String;
}

impl Hex for Vec<u8> {
    fn hex(&self) -> String {
        self.iter()
            .map(|x| -> String { format!("{:02x}", x) })
            .collect()
    }
}

impl Hex for &[u8] {
    fn hex(&self) -> String {
        self.iter()
            .map(|x| -> String { format!("{:02x}", x) })
            .collect()
    }
}

#[derive(Clone)]
pub struct Cache {
    con: redis::aio::ConnectionManager,
    root: PathBuf,
}

impl Cache {
    pub async fn new<S, P>(url: S, root: P) -> Result<Cache>
    where
        S: AsRef<str>,
        P: Into<PathBuf>,
    {
        let client = Client::open(url.as_ref())?;
        let mgr = client
            .get_tokio_connection_manager()
            .await
            .context("failed to open connection to storage")?;
        Ok(Cache {
            con: mgr,
            root: root.into(),
        })
    }

    // get content from redis
    async fn get_data(&mut self, id: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        let result: Vec<u8> = redis::cmd("GET").arg(id).query_async(&mut self.con).await?;
        if result.is_empty() {
            bail!("invalid chunk length downloaded");
        }

        let key = unsafe { std::str::from_utf8_unchecked(key) };
        let mut decoder = snap::raw::Decoder::new();
        let result = match decoder.decompress_vec(&xxtea::decrypt(&result, key)) {
            Ok(data) => data,
            Err(_) => bail!("invalid chunk"),
        };

        Ok(result)
    }

    // download given an open file, writes the content of the chunk to the file
    async fn download(&mut self, file: &mut File, block: &FileBlock) -> Result<u64> {
        file.rewind().await?;
        let data = self.get_data(&block.hash, &block.key).await?;
        file.write_all(&data).await?;

        Ok(data.len() as u64)
    }

    async fn prepare(&self, id: &[u8]) -> Result<File> {
        let name = id.hex();
        if name.len() < 4 {
            bail!("invalid chunk hash");
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

    pub async fn get(&mut self, block: &FileBlock) -> Result<(u64, File)> {
        let mut file = self.prepare(&block.hash).await?;
        // TODO: locking must happen here so no
        // other processes start downloading the same chunk
        let locker = Locker::new(&file);
        locker.lock().await?;

        let meta = file.metadata().await?;
        if meta.len() > 0 {
            // chunk is already downloaded
            debug!("block cache hit: {}", block.hash.hex());
            locker.unlock().await?;
            return Ok((meta.len(), file));
        }

        debug!("downloading block: {}", block.hash.hex());
        let size = self.download(&mut file, block).await?;

        locker.unlock().await?;
        Ok((size, file))
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
        let fd = self.fd.clone();
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