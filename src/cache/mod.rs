use crate::meta::types::FileBlock;
use anyhow::{Context, Result};
use bb8_redis::redis::aio::Connection;
use bb8_redis::{
    bb8::{CustomizeConnection, Pool},
    redis::{cmd, AsyncCommands, RedisError},
    RedisConnectionManager,
};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

trait Hex {
    fn hex(&self) -> String;
}

impl Hex for &[u8] {
    fn hex(&self) -> String {
        self.iter()
            .map(|x| -> String { format!("{:02x}", x) })
            .collect()
    }
}

#[derive(Debug)]
struct WithNamespace {
    namespace: Option<String>,
}

#[async_trait::async_trait]
impl CustomizeConnection<Connection, RedisError> for WithNamespace {
    async fn on_acquire(&self, connection: &mut Connection) -> Result<(), RedisError> {
        match self.namespace {
            Some(ref ns) if ns != "default" => {
                let result = cmd("SELECT").arg(ns).query_async(connection).await;
                if let Err(ref err) = result {
                    error!("failed to switch namespace to {}: {}", ns, err);
                }
                result
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone)]
pub struct Cache {
    pool: Pool<RedisConnectionManager>,
    root: PathBuf,
}

impl Cache {
    pub async fn new<S, P>(url: S, root: P) -> Result<Cache>
    where
        S: AsRef<str>,
        P: Into<PathBuf>,
    {
        let mut u: url::Url = url.as_ref().parse().context("failed to parse url")?;
        let namespace: Option<String> = match u.path_segments() {
            None => None,
            Some(mut segments) => segments.next().map(|s| s.to_owned()),
        };

        u.set_path("");

        let mgr = RedisConnectionManager::new(u)?;
        let namespace = WithNamespace {
            namespace: namespace,
        };
        let pool = Pool::builder()
            .max_size(20)
            .connection_customizer(Box::new(namespace))
            .build(mgr)
            .await?;

        Ok(Cache {
            pool,
            root: root.into(),
        })
    }

    // get content from redis
    async fn get_data(&self, id: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        let mut con = self.pool.get().await.context("failed to get connection")?;
        //con.
        let result: Vec<u8> = con.get(id).await?;
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
    async fn download(&self, file: &mut File, block: &FileBlock) -> Result<u64> {
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

    /// get a file block either from cache or from remote if it's already
    /// not cached
    pub async fn get(&self, block: &FileBlock) -> Result<(u64, File)> {
        let mut file = self.prepare(&block.hash).await?;
        // TODO: locking must happen here so no
        // other processes start downloading the same chunk
        let locker = Locker::new(&file);
        locker.lock().await?;

        let meta = file.metadata().await?;
        if meta.len() > 0 {
            // chunk is already downloaded
            debug!("block cache hit: {}", block.hash.as_slice().hex());
            locker.unlock().await?;
            return Ok((meta.len(), file));
        }

        debug!("downloading block: {}", block.hash.as_slice().hex());
        let size = self.download(&mut file, block).await?;

        // if file is just downloaded, we need
        // to seek to beginning of the file.
        file.rewind().await?;

        locker.unlock().await?;
        Ok((size, file))
    }

    /// direct downloads all the file blocks from remote and write it to output
    #[allow(dead_code)]
    pub async fn direct(&self, blocks: &[FileBlock], out: &mut File) -> Result<()> {
        use tokio::io::copy;
        for (index, block) in blocks.iter().enumerate() {
            let (_, mut chunk) = self.get(&block).await?;
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
