mod bs;
pub mod dir;
pub mod http;
mod router;
pub mod s3store;
pub mod server;
pub mod zdb;

use anyhow::Context;
use rand::seq::SliceRandom;

pub use bs::BlockStore;
use regex::Regex;

use crate::fungi;

pub use self::router::Router;

pub async fn make<U: AsRef<str>>(u: U) -> Result<Stores> {
    let parsed = url::Url::parse(u.as_ref())?;

    match parsed.scheme() {
        dir::SCHEME => return Ok(Stores::Dir(dir::DirStore::make(&u).await?)),
        "s3" | "s3s" | "s3s+tls" => return Ok(Stores::S3(s3store::S3Store::make(&u).await?)),
        "zdb" => return Ok(Stores::ZDB(zdb::ZdbStore::make(&u).await?)),
        "http" | "https" => return Ok(Stores::HTTP(http::HTTPStore::make(&u).await?)),
        server::SCHEME => return Ok(Stores::Server(server::ServerStore::make(&u).await?)),
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

pub async fn get_router(meta: &fungi::Reader) -> Result<Router<Stores>> {
    let mut router = Router::new();

    for route in meta.routes().await.context("failed to get store routes")? {
        let store = make(&route.url)
            .await
            .with_context(|| format!("failed to initialize store '{}'", route.url))?;
        router.add(route.start, route.end, store);
    }

    Ok(router)
}

pub async fn parse_router(urls: &[String]) -> anyhow::Result<Router<Stores>> {
    let mut router = Router::new();
    let pattern = r"^(?P<range>[0-9a-f]{2}-[0-9a-f]{2})=(?P<url>.+)$";
    let re = Regex::new(pattern)?;

    for u in urls {
        let ((start, end), store) = match re.captures(u) {
            None => ((0x00, 0xff), make(u).await?),
            Some(captures) => {
                let url = captures.name("url").context("missing url group")?.as_str();
                let rng = captures
                    .name("range")
                    .context("missing range group")?
                    .as_str();

                let store = make(url).await?;
                let range = match rng.split_once('-') {
                    None => anyhow::bail!("invalid range format"),
                    Some((low, high)) => (
                        u8::from_str_radix(low, 16)
                            .with_context(|| format!("failed to parse low range '{}'", low))?,
                        u8::from_str_radix(high, 16)
                            .with_context(|| format!("failed to parse high range '{}'", high))?,
                    ),
                };
                (range, store)
            }
        };

        router.add(start, end, store);
    }

    Ok(router)
}

#[derive(Clone)]
pub enum Stores {
    S3(s3store::S3Store),
    Dir(dir::DirStore),
    ZDB(zdb::ZdbStore),
    HTTP(http::HTTPStore),
    Server(server::ServerStore),
}

#[async_trait::async_trait]
impl Store for Stores {
    async fn get(&self, key: &[u8]) -> Result<Vec<u8>> {
        match self {
            self::Stores::S3(s3_store) => s3_store.get(key).await,
            self::Stores::Dir(dir_store) => dir_store.get(key).await,
            self::Stores::ZDB(zdb_store) => zdb_store.get(key).await,
            self::Stores::HTTP(http_store) => http_store.get(key).await,
            self::Stores::Server(server_store) => server_store.get(key).await,
        }
    }
    async fn set(&self, key: &[u8], blob: &[u8]) -> Result<()> {
        match self {
            self::Stores::S3(s3_store) => s3_store.set(key, blob).await,
            self::Stores::Dir(dir_store) => dir_store.set(key, blob).await,
            self::Stores::ZDB(zdb_store) => zdb_store.set(key, blob).await,
            self::Stores::HTTP(http_store) => http_store.set(key, blob).await,
            self::Stores::Server(server_store) => server_store.set(key, blob).await,
        }
    }
    fn routes(&self) -> Vec<Route> {
        match self {
            self::Stores::S3(s3_store) => s3_store.routes(),
            self::Stores::Dir(dir_store) => dir_store.routes(),
            self::Stores::ZDB(zdb_store) => zdb_store.routes(),
            self::Stores::HTTP(http_store) => http_store.routes(),
            self::Stores::Server(server_store) => server_store.routes(),
        }
    }
}
