mod bs;
pub mod dir;
mod router;
pub mod s3store;
pub mod zdb;

use rand::seq::SliceRandom;

pub use bs::BlockStore;

pub use self::router::Router;

pub async fn make<U: AsRef<str>>(u: U) -> Result<Stores> {
    let parsed = url::Url::parse(u.as_ref())?;

    match parsed.scheme() {
        dir::SCHEME => return Ok(Stores::Dir(
            dir::DirStore::make(&u)
                .await
                .expect("failed to make dir store"),
        )),
        "s3" | "s3s" | "s3s+tls" => return Ok(Stores::S3(
            s3store::S3Store::make(&u)
                .await
                .expect(format!("failed to make {} store", parsed.scheme()).as_str()),
        )),
        "zdb" => return Ok(Stores::ZDB(
            zdb::ZdbStore::make(&u)
                .await
                .expect("failed to make zdb store"),
        )),
        _ => return Err(Error::UnknownStore(parsed.scheme().into())),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("key not found")]
    KeyNotFound,
    #[error("invalid key")]
    InvalidKey,
    #[error("invalid blob")]
    InvalidBlob,
    #[error("key is not routable")]
    KeyNotRoutable,
    #[error("store is not available")]
    Unavailable,

    #[error("compression error: {0}")]
    Compression(#[from] snap::Error),

    #[error("encryption error")]
    EncryptionError,

    // TODO: better display for the Box<Vec<Self>>
    #[error("multiple error: {0:?}")]
    Multiple(Box<Vec<Self>>),

    #[error("io error: {0}")]
    IO(#[from] std::io::Error),

    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("unknown store type '{0}'")]
    UnknownStore(String),
    #[error("invalid schema '{0}' expected '{1}'")]
    InvalidScheme(String, String),

    #[error("unknown store error {0:#}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Route {
    pub start: Option<u8>,
    pub end: Option<u8>,
    pub url: String,
}

impl Route {
    pub fn url<S: Into<String>>(s: S) -> Self {
        Self {
            start: None,
            end: None,
            url: s.into(),
        }
    }
}
/// The store trait defines a simple (low level) key/value store interface to set/get blobs
/// the concern of the store is to only store given data with given key and implement
/// the means to retrieve it again once a get is called.
#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>>;
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()>;
    fn routes(&self) -> Vec<Route>;
}

#[async_trait::async_trait]
impl<S> Store for Router<S>
where
    S: Store,
{
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        if key.is_empty() {
            return Err(Error::InvalidKey);
        }
        let mut errors = Vec::default();

        // to make it fare we shuffle the list of matching routers randomly everytime
        // before we do a get
        let mut routers: Vec<&S> = self.route(key[0]).collect();
        routers.shuffle(&mut rand::thread_rng());
        for store in routers {
            match store.get(key).await {
                Ok(object) => return Ok(object),
                Err(err) => errors.push(err),
            };
        }

        if errors.is_empty() {
            return Err(Error::KeyNotRoutable);
        }

        // return aggregated errors
        return Err(Error::Multiple(Box::new(errors)));
    }

    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        if key.is_empty() {
            return Err(Error::InvalidKey);
        }

        let mut b = false;
        for store in self.route(key[0]) {
            b = true;
            store.set(key, blob).await?;
        }

        if !b {
            return Err(Error::KeyNotRoutable);
        }

        Ok(())
    }

    fn routes(&self) -> Vec<Route> {
        let mut routes = Vec::default();
        for (key, value) in self.routes.iter() {
            for sub in value.routes() {
                let r = Route {
                    start: Some(sub.start.unwrap_or(*key.start())),
                    end: Some(sub.end.unwrap_or(*key.end())),
                    url: sub.url,
                };
                routes.push(r);
            }
        }

        routes
    }
}
pub enum Stores {
    S3(s3store::S3Store),
    Dir(dir::DirStore),
    ZDB(zdb::ZdbStore),
}

#[async_trait::async_trait]
impl Store for Stores {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        match self {
            self::Stores::S3(s3_store) => s3_store.get(key).await,
            self::Stores::Dir(dir_store) => dir_store.get(key).await,
            self::Stores::ZDB(zdb_store) => zdb_store.get(key).await,
        }
    }
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        match self {
            self::Stores::S3(s3_store) => s3_store.set(key, blob).await,
            self::Stores::Dir(dir_store) => dir_store.set(key, blob).await,
            self::Stores::ZDB(zdb_store) => zdb_store.set(key, blob).await,
        }
    }
    fn routes(&self) -> Vec<Route> {
        match self {
            self::Stores::S3(s3_store) => s3_store.routes(),
            self::Stores::Dir(dir_store) => dir_store.routes(),
            self::Stores::ZDB(zdb_store) => zdb_store.routes(),
        }
    }
}
