use crate::meta::types::FileBlock;
use anyhow::Result;
use redis::Client;
use std::path::PathBuf;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

trait Hex {
    fn hex(self: &Self) -> String;
}

impl Hex for Vec<u8> {
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
        let mgr = client.get_tokio_connection_manager().await?;
        Ok(Cache {
            con: mgr,
            root: root.into(),
        })
    }

    // get content from redis
    async fn get_data(&mut self, id: &Vec<u8>, key: &Vec<u8>) -> Result<Vec<u8>> {
        let result: Vec<u8> = redis::cmd("GET").arg(id).query_async(&mut self.con).await?;
        if result.len() == 0 {
            bail!("invalid chunk length downloaded");
        }

        let key = unsafe { std::str::from_utf8_unchecked(key) };
        let result = match snappy::uncompress(&xxtea::decrypt(&result, key)) {
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

    async fn prepare(&self, id: &Vec<u8>) -> Result<File> {
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

        let meta = file.metadata().await?;
        if meta.len() > 0 {
            // chunk is already downloaded
            debug!("cache hit: {}", block.hash.hex());
            return Ok((meta.len(), file));
        }
        let size = self.download(&mut file, block).await?;
        //file.rewind().await?;

        Ok((size, file))
    }
}
