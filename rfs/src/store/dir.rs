use super::{Error, Result, Route, Store};
use std::io::ErrorKind;
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use tokio::fs;
use url;

pub const SCHEME: &str = "dir";

/// DirStore is a simple store that store blobs on the filesystem
/// and is mainly used for testing

#[derive(Clone)]
pub struct DirStore {
    root: PathBuf,
}

impl DirStore {
    pub async fn make<U: AsRef<str>>(url: &U) -> Result<DirStore> {
        let u = url::Url::parse(url.as_ref())?;
        if u.scheme() != SCHEME {
            return Err(Error::InvalidScheme(u.scheme().into(), SCHEME.into()));
        }

        Ok(DirStore::new(u.path()).await?)
    }
    pub async fn new<P: Into<PathBuf>>(root: P) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }
}

#[async_trait::async_trait]
impl Store for DirStore {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        let file_name = hex::encode(key);
        let dir_path = self.root.join(&file_name[0..2]);

        let path = match fs::try_exists(dir_path.clone()).await {
            Ok(true) => dir_path.join(file_name),
            Ok(false) => self.root.join(file_name),
            Err(e) => return Err(Error::IO(e)),
        };

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
        let file_name = hex::encode(key);
        let dir_path = self.root.join(&file_name[0..2]);

        fs::create_dir_all(dir_path.clone()).await?;

        let file_path = dir_path.join(file_name);
        fs::write(file_path, blob).await?;
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
