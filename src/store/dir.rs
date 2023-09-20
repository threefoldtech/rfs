use super::{Error, Result, Route, Store};
use std::io::ErrorKind;
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use tokio::fs;

/// DirStore is a simple store that store blobs on the filesystem
/// and is mainly used for testing

pub struct DirStore {
    root: PathBuf,
}

impl DirStore {
    pub async fn new<P: Into<PathBuf>>(root: P) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }
}

#[async_trait::async_trait]
impl Store for DirStore {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        let path = self.root.join(hex::encode(key));
        let data = match fs::read(&path).await {
            Ok(data) => data,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                return Err(Error::KeyNotFound);
            }
            Err(err) => {
                return Err(Error::IO(err));
            }
        };

        Ok(data)
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        let path = self.root.join(hex::encode(key));

        fs::write(path, blob).await?;
        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        let r = Route::url(format!(
            "dir://{}",
            String::from_utf8_lossy(self.root.as_os_str().as_bytes())
        ));

        vec![r]
    }
}
